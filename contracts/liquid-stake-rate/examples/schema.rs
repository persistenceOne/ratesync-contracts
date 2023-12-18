use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use liquid_stake_rate::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, LiquidStakeRateResponse, LiquidStakeRates},
    state::{Config, LiquidStakeRate},
};
use std::env::current_dir;
use std::fs::create_dir_all;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(LiquidStakeRate), &out_dir);
    export_schema(&schema_for!(LiquidStakeRates), &out_dir);
    export_schema(&schema_for!(LiquidStakeRateResponse), &out_dir);
}