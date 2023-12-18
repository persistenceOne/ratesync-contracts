use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::LiquidStakeRate;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Admin address
    pub admin: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Set c-value for denom pair
    LiquidStakeRate {
        /// Default bond denom
        default_bond_denom: String,
        /// Stake denom
        stk_denom: String,
        /// Exchange rate for denom pair
        c_value: Decimal,
        /// time
        controller_chain_time: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(ConfigResponse)]
    Config {},

    /// Returns the liquid stake rate of an stToken
    #[returns(LiquidStakeRateResponse)]
    LiquidStakeRate {
        /// Default bond denom
        default_bond_denom: String,
        /// Stake denom
        stk_denom: String,
    },

    /// Returns a list of liquid stake rates over time for an stToken
    #[returns(LiquidStakeRates)]
    HistoricalLiquidStakeRates {
        /// Default bond denom
        default_bond_denom: String,
        /// Stake denom
        stk_denom: String,
        /// Optional limit on the number of entries to return
        limit: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ConfigParams {
    /// Owner address for config update
    pub owner: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct LiquidStakeRateResponse {
    pub c_value: Decimal,
    pub last_updated: u64,
}

#[cw_serde]
pub struct LiquidStakeRates {
    pub c_value_rates: Vec<LiquidStakeRate>,
}
