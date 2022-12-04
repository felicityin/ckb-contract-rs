use std::error::Error as StdErr;
use std::fs;

use anyhow::Result;

use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient,
    traits::{
        DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver,
        DefaultTransactionDependencyProvider
    },
    tx_builder::{ScriptGroups, gen_script_groups, balance_tx_capacity, CapacityBalancer},
    unlock::MultisigConfig,
    ScriptId,
};
use ckb_types::{
    core::{BlockView, Capacity, TransactionView},
    packed::{CellOutput, Script, OutPoint, CellInput},
    prelude::*,
    core::TransactionBuilder,
};

use super::multisig_account::{create_multisig_script};
use super::utils::send_tx;
use super::multisig_deploy::{sign_tx, build_multisig_unlockers, load_contract};
use super::cell_collector;

pub fn upgrade_contract(
    contract_out_point: OutPoint,
    sender_keys: Vec<secp256k1::SecretKey>,
    multisig_config: &MultisigConfig,
    ckb_rpc: &str,
    new_contract_file: &str,
) {
    let tx = build_upgrade_tx(
        contract_out_point,
        multisig_config,
        ckb_rpc,
        new_contract_file
    ).expect("build tx");
    let tx = sign_tx(tx, multisig_config, sender_keys, ckb_rpc).expect("sign tx");
    let tx_hash = send_tx(ckb_rpc, json_types::TransactionView::from(tx).inner);
    println!("tx sent: {:?}", tx_hash);
}

pub fn build_upgrade_tx(
    contract_out_point: OutPoint,
    multisig_config: &MultisigConfig,
    ckb_rpc: &str,
    new_binary_path: &str,
) -> Result<TransactionView, Box<dyn StdErr>> {
    let mut ckb_client = CkbRpcClient::new(ckb_rpc);
    let lock_script = create_multisig_script(multisig_config);

    // input: old contract
    let old_contract_cell = CellInput::new(
        contract_out_point.clone(),
        0,
    );

    let cell = cell_collector::get_live_cell(
        &mut ckb_client,
        contract_out_point,
        true
    ).expect("cell has been spent").expect("none cell");
    let type_script = Script::from(cell.output.type_.expect("none type script"));

    // output: new contract
    let file_size = fs::metadata(new_binary_path)?.len();
    let min_output_capacity = {
        let data_capacity = Capacity::bytes(file_size as usize)?;
        let output = CellOutput::new_builder()
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        output.occupied_capacity(data_capacity)?.as_u64()
    };
    let new_contract_output = CellOutput::new_builder()
        .capacity(Capacity::shannons(min_output_capacity).pack())
        .lock(lock_script.clone())
        .type_(Some(type_script).pack())
        .build();

    // output data
    let new_contract_bin = load_contract(new_binary_path).expect("load contract");

    // tx
    let mut tx = TransactionBuilder::default()
        .set_inputs(vec![old_contract_cell])
        .set_outputs(vec![new_contract_output])
        .set_outputs_data(vec![new_contract_bin.pack()])
        .build();

    // witness
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(ckb_rpc, 10);
    let unlockers = build_multisig_unlockers(Vec::new(), multisig_config.clone());
    // let tx = ckb_sdk::tx_builder::fill_placeholder_witnesses(tx, &tx_dep_provider, &unlockers).unwrap();  // wrong!!!!! witness won't be filled
    let ScriptGroups { lock_groups, .. } = gen_script_groups(&tx, &tx_dep_provider)?;
    for lock_group in lock_groups.values() {
        let script_id = ScriptId::from(&lock_group.script);
        tx = unlockers.get(&script_id).unwrap().fill_placeholder_witness(&tx, lock_group, &tx_dep_provider)?;
    }

    // balance tx
    let mut cell_collector = DefaultCellCollector::new(ckb_rpc);
    let cell_dep_resolver = {
        let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
        DefaultCellDepResolver::from_genesis(&BlockView::from(genesis_block))?
    };
    let header_dep_resolver = DefaultHeaderDepResolver::new(ckb_rpc);
    let placeholder_witness = multisig_config.placeholder_witness();
    let balancer = CapacityBalancer::new_simple(lock_script.clone(), placeholder_witness, 1000);
    Ok(balance_tx_capacity(
        &tx,
        &balancer,
        &mut cell_collector,
        &tx_dep_provider,
        &cell_dep_resolver,
        &header_dep_resolver,
    )?)
}
