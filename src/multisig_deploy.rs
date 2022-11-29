use std::collections::HashMap;
use std::error::Error as StdErr;
use std::fs;
use std::io::Read;

use anyhow::Result;

use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    constants::{MULTISIG_TYPE_HASH, TYPE_ID_CODE_HASH},
    rpc::CkbRpcClient,
    traits::{
        DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver,
        DefaultTransactionDependencyProvider, SecpCkbRawKeySigner,
    },
    tx_builder::{transfer::CapacityTransferBuilder, CapacityBalancer, TxBuilder, unlock_tx},
    unlock::{MultisigConfig, ScriptUnlocker, SecpMultisigScriptSigner, SecpMultisigUnlocker},
    ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, Capacity, ScriptHashType, TransactionView},
    packed::{CellOutput, Script},
    prelude::*,
};

use super::multisig_account::{create_multisig_script};
use super::utils::{build_type_id_script, send_tx};

pub fn deploy_contract(
    sender_keys: Vec<secp256k1::SecretKey>, 
    multisig_config: &MultisigConfig,
    ckb_rpc: &str,
    contract_file: &str,
) {
    let tx = build_deploy_tx(multisig_config, ckb_rpc, contract_file).expect("build tx");
    let tx = sign_tx(tx, multisig_config, sender_keys, ckb_rpc).expect("sign tx");
    send_tx(ckb_rpc, json_types::TransactionView::from(tx).inner);
}

pub fn build_deploy_tx(
    multisig_config: &MultisigConfig,
    ckb_rpc: &str,
    binary_path: &str,
) -> Result<TransactionView, Box<dyn StdErr>> {
    let lock_script = create_multisig_script(multisig_config);

    let type_script = Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(Bytes::from(vec![0u8; 32]).pack())
        .build();

    let file_size = fs::metadata(binary_path)?.len();
    let min_output_capacity = {
        let data_capacity = Capacity::bytes(file_size as usize)?;
        let output = CellOutput::new_builder()
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build();
        output.occupied_capacity(data_capacity)?.as_u64()
    };

    let contract_output = CellOutput::new_builder()
        .capacity(Capacity::shannons(min_output_capacity).pack())
        .lock(lock_script.clone())
        .type_(Some(type_script).pack())
        .build();

    let contract_bin = load_contract(binary_path).expect("load contract");

    let builder = CapacityTransferBuilder::new(vec![(contract_output, contract_bin)]);
    
    let mut cell_collector = DefaultCellCollector::new(ckb_rpc);
    let mut ckb_client = CkbRpcClient::new(ckb_rpc);
    let cell_dep_resolver = {
        let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
        DefaultCellDepResolver::from_genesis(&BlockView::from(genesis_block))?
    };
    let header_dep_resolver = DefaultHeaderDepResolver::new(ckb_rpc);
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(ckb_rpc, 10);
    let placeholder_witness = multisig_config.placeholder_witness();
    let balancer = CapacityBalancer::new_simple(lock_script.clone(), placeholder_witness, 1000);
    let unlockers = build_multisig_unlockers(Vec::new(), multisig_config.clone());

    let mut tx = builder
        .build_balanced(
            &mut cell_collector,
            &cell_dep_resolver,
            &header_dep_resolver,
            &tx_dep_provider,
            &balancer,
            &unlockers,
        )
        .map_err(|err| err.to_string())?;

    let first_cell_input = tx.inputs().into_iter().next().expect("inputs empty");
    let type_script = build_type_id_script(&first_cell_input, 0);
    let mut outputs = tx.outputs().into_iter().collect::<Vec<_>>();
    outputs[0] = tx
        .output(0)
        .expect("first output")
        .as_builder()
        .type_(Some(type_script).pack())
        .build();
    tx = tx.as_advanced_builder().set_outputs(outputs).build();

    Ok(tx)
}

pub fn sign_tx(
    mut tx: TransactionView,
    multisig_config: &MultisigConfig,
    sender_keys: Vec<secp256k1::SecretKey>,
    ckb_rpc: &str,
) -> Result<TransactionView> {
    // Unlock transaction
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(ckb_rpc, 10);
    for key in sender_keys {
        let unlockers = build_multisig_unlockers(vec![key], multisig_config.clone());
        let (new_tx, _new_still_locked_groups) =
            unlock_tx(tx.clone(), &tx_dep_provider, &unlockers)?;
        tx = new_tx;
    }
    Ok(tx)
}

pub fn load_contract(file: &str) -> Result<Bytes> {
    let mut data = Vec::new();
    match fs::File::open(&file).and_then(|mut f| f.read_to_end(&mut data)) {
        Ok(_) => return Ok(data.into()),
        Err(err) => {
            eprintln!("failed to read cell data from '{}', err: {}", file, &err);
            return Err(err.into());
        }
    }
}

pub fn build_multisig_unlockers(
    keys: Vec<secp256k1::SecretKey>,
    config: MultisigConfig,
) -> HashMap<ScriptId, Box<dyn ScriptUnlocker>> {
    let signer = SecpCkbRawKeySigner::new_with_secret_keys(keys);
    let multisig_signer = SecpMultisigScriptSigner::new(Box::new(signer), config);
    let multisig_unlocker = SecpMultisigUnlocker::new(multisig_signer);
    let multisig_script_id = ScriptId::new_type(MULTISIG_TYPE_HASH.clone());
    let mut unlockers = HashMap::default();
    unlockers.insert(
        multisig_script_id,
        Box::new(multisig_unlocker) as Box<dyn ScriptUnlocker>,
    );
    unlockers
}
