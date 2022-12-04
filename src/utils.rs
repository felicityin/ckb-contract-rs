use std::str::FromStr;

use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient,
    constants::TYPE_ID_CODE_HASH,
    Address,
    NetworkType,
    AddressPayload, 
};
use ckb_hash::{new_blake2b};
use ckb_jsonrpc_types::Transaction;
use ckb_types::{
    bytes::Bytes,
    core::ScriptHashType,
    packed::{CellInput, Script},
    prelude::*,
};
use molecule::prelude::Entity;

pub fn send_tx(ckb_rpc: &str, tx: Transaction) -> String {
    let outputs_validator = Some(json_types::OutputsValidator::Passthrough);
    let tx_hash = CkbRpcClient::new(ckb_rpc)
        .send_transaction(tx, outputs_validator)
        .expect("send transaction");
    tx_hash.to_string()
}

pub fn build_type_id_script(input: &CellInput, output_index: u64) -> Script {
    let mut blake2b = new_blake2b();
    blake2b.update(input.as_slice());
    blake2b.update(&output_index.to_le_bytes());
    let mut ret = [0; 32];
    blake2b.finalize(&mut ret);
    let script_arg = Bytes::from(ret.to_vec());
    Script::new_builder()
        .code_hash(TYPE_ID_CODE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(script_arg.pack())
        .build()
}

pub fn to_address(script: Script, network: NetworkType) -> Address {
    Address::new(network, script.into(), true)
}

pub fn address_from_script(slice: &[u8], network: NetworkType) -> Address {
    let payload =
        AddressPayload::from(Script::from_slice(slice).expect("address_from_script"));
    Address::new(network, payload, true)
}

pub fn script_from_address(address: String) -> Script {
    let addr = Address::from_str(&address).expect("script_from_address");
    let payload = addr.payload();
    Script::new_builder()
        .hash_type(payload.hash_type().into())
        .code_hash(payload.code_hash(None))
        .args(payload.args().pack())
        .build()
}
