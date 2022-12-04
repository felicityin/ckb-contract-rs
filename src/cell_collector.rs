use anyhow::{anyhow, Result};

use ckb_jsonrpc_types as json_types;
use json_types::CellInfo;
use ckb_sdk::{
    rpc::{
        CkbRpcClient,
    },
};
use ckb_types::{
    packed::OutPoint,
};

pub fn get_live_cell(
    client: &mut CkbRpcClient,
    out_point: OutPoint,
    with_data: bool,
) -> Result<Option<CellInfo>> {
    let cell = client
        .get_live_cell(out_point.clone().into(), with_data)
        .unwrap();
    if cell.status != "live" {
        return Err(anyhow!(
            "Invalid cell status: {}, out_point: {}",
            cell.status, out_point
        ));
    }
    Ok(cell.cell)
}
