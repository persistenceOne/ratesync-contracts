use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Contract configuration
    pub config: ConfigParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update contract config (only owner can call)
    UpdateConfig { config: ConfigParams },

    /// Accept ownership of the contract (only pending owner can call)
    /// This is used to transfer ownership of the contract to a new address
    AcceptOwnership {},

    /// Cancel ownership transfer (only pending owner can call)
    /// This is used to cancel a pending ownership transfer
    CancelOwnership {},

    /// Set redemption rate for denom pair (only owner can call)
    SetRedemptionRate {
        /// Denom pairs
        price: Price,
        /// Exchange rate for denom pair
        exchange_rate: Decimal,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(ConfigResponse)]
    Config {},

    // Get redemption rate
    #[returns(RedemptionRateResponse)]
    RedemptionRateRequest { price: Price },
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Price {
    pub denom: String,
    pub base_denom: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct RedemptionRateResponse {
    pub exchange_rate: Decimal,
    pub last_updated: u64,
}
