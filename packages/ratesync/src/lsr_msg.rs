use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::lsr_state::RedemptionRate;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Admin address
    pub admin: Option<String>,
    /// Transfer Channel ID
    pub transfer_channel_i_d: String,
    /// Transfer Port ID
    pub transfer_port_i_d: String,
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

    /// Returns the redemption rate of an stkToken
    #[returns(RedemptionRateResponse)]
    RedemptionRate {
        /// The denom should be the ibc hash of an stkToken as it lives on the oracle chain
        /// (e.g. ibc/{hash(transfer/channel-0/stkuatom)} on Osmosis)
        denom: String,
        /// Params should always be None, but was included in this query
        /// to align with other price oracles that take additional parameters such as TWAP
        params: Option<Binary>,
    },

    /// Returns a list of redemption rates over time for an stkToken
    #[returns(RedemptionRates)]
    HistoricalRedemptionRates {
        /// The denom should be the ibc hash of an stkToken as it lives on the oracle chain
        /// (e.g. ibc/{hash(transfer/channel-0/stkuatom)} on Osmosis)
        denom: String,
        /// Params should always be None, but was included in this query
        /// to align with other price oracles that take additional parameters such as TWAP
        params: Option<Binary>,
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

#[cw_serde]
pub struct RedemptionRateResponse {
    pub redemption_rate: Decimal,
    pub update_time: u64,
}

#[cw_serde]
pub struct RedemptionRates {
    pub redemption_rates: Vec<RedemptionRate>,
}
