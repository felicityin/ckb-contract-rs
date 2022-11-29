use anyhow::{anyhow, Result};

use ckb_sdk::{
    constants::{SIGHASH_TYPE_HASH, MULTISIG_TYPE_HASH},
    unlock::{MultisigConfig},
    Address,
    NetworkType,
};
use ckb_types::{
    bytes::Bytes,
    core::{ScriptHashType},
    H160,
    prelude::{Builder, Entity, Pack},
    packed::{Script},
};

use super::utils::to_address;

pub fn create_multisig_config(require_first_n: u8, threshold: u8, sighash_address: Vec<Address>) -> Result<MultisigConfig> {
    let mut sighash_addresses = Vec::with_capacity(sighash_address.len());
    for addr in sighash_address.clone() {
        let lock_args = addr.payload().args();
        if addr.payload().code_hash(None).as_slice() != SIGHASH_TYPE_HASH.as_bytes()
            || addr.payload().hash_type() != ScriptHashType::Type
            || lock_args.len() != 20
        {
            return Err(anyhow!(
                format!("sighash_address {} is not sighash address", addr)
            ));
        }
        sighash_addresses.push(H160::from_slice(lock_args.as_ref()).unwrap());
    }
    Ok(MultisigConfig::new_with(sighash_addresses, require_first_n, threshold)?)
}

pub fn create_multisig_script(multisig_config: &MultisigConfig) -> Script {
    Script::new_builder()
        .code_hash(MULTISIG_TYPE_HASH.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(Bytes::from(multisig_config.hash160().as_bytes().to_vec()).pack())
        .build()
}

pub fn create_multisig_address(multisig_config: &MultisigConfig, network: NetworkType) -> Address {
    let script = create_multisig_script(multisig_config);
    to_address(script, network)
}
