use std::error::Error as StdErr;
use std::str::FromStr;

use anyhow::Result;

use ckb_sdk::{
    unlock::{MultisigConfig},
    Address, NetworkType
};
use ckb_types::{
    H256, h256,
};

mod multisig_account;
mod multisig_deploy;
mod utils;

use utils::{to_address};
use multisig_account::{create_multisig_config, create_multisig_script};

const CKB_RPC: &str = "https://testnet.ckb.dev/";

const REQURE_FIRST_N: u8 = 2;
const THRESHOLD: u8 = 2;
const PRIVATE_KEYS: [H256; 2] = [
    h256!("0x13b08bb054d5dd04013156dced8ba2ce4d8cc5973e10d905a228ea1abc267e60"),
    h256!("0x5368b818f59570b5bc078a6a564f098a191dcb8938d95c413be5065fd6c42d32"),
];
const SIGHADH_ADDRESS: [&str; 2] = [
    "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqw0q40f6t2slk2pyraxmxqh9z5mu4dl7wc2y595g",
    "ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqdkmkag0w667hc98vdwt09u0ax7qdre7lsgje9su",
];

const CONTRACT_FILE: &str = "./contract-bin/always-success";

fn main() -> Result<(), Box<dyn StdErr>> {
    let sighash_address: Vec<Address> = SIGHADH_ADDRESS
        .iter()
        .map(|addr| Address::from_str(addr.to_owned()).expect("addr"))
        .collect();
    let multisig_config = create_multisig_config(REQURE_FIRST_N, THRESHOLD, sighash_address)?;

    create_multisig_address(&multisig_config);

    multisig_deploy::deploy_contract(get_keys(), &multisig_config, CKB_RPC, CONTRACT_FILE);

    Ok(())
}

fn create_multisig_address(multisig_config: &MultisigConfig) {
    let sender_script = create_multisig_script(multisig_config);
    let sender_addr = to_address(sender_script.clone(), NetworkType::Testnet);
    println!("multisig address: {}", sender_addr);
}

fn get_keys() -> Vec<secp256k1::SecretKey> {
    return PRIVATE_KEYS
        .iter()
        .map(|key| secp256k1::SecretKey::from_slice(key.to_owned().as_bytes()).expect("invalid key"))
        .collect();
}
