use crate::state::Pool;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Decimal};

use crate::state::AssetOrdering;

/// Instantiates the contract with an admin address and lsr contract address
#[cw_serde]
pub struct InstantiateMsg {
    pub owner_address: String,
    pub lsr_contract_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        owner_address: String,
        lsr_contract_address: String,
    },
    AddPool {
        /// Pool ID of the Osmosis pool
        pool_id: u64,
        /// The denom of the stkToken as it lives on Osmosis
        stk_token_denom: String,
        /// The transfer port id
        transfer_port_id: String,
        /// The transfer channel id
        transfer_channel_id: String,
        /// The ordering of the stkToken vs nativeToken assets in the Osmosis pool,
        asset_ordering: AssetOrdering,
    },
    RemovePool {
        pool_id: u64,
    },

    UpdateScalingFactor {
        pool_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the contract's config
    #[returns(crate::state::Config)]
    Config {},

    /// Returns a the configuration for a specific stkToken stableswap pool
    #[returns(crate::state::Pool)]
    Pool { pool_id: u64 },

    /// Returns all pools controlled by the contract
    #[returns(Pools)]
    AllPools {},
}

#[cw_serde]
pub struct Pools {
    pub pools: Vec<Pool>,
}

/// RedemptionRate query as defined in the LiquidStakeRate contract
#[cw_serde]
#[derive(QueryResponses)]
pub enum LiquidStakeRateQueryMsg {
    #[returns(RedemptionRateResponse)]
    RedemptionRate {
        denom: String,
        params: Option<Binary>,
    },
}

/// Response from LiquidStakeRate contract redemption rate query
#[cw_serde]
pub struct RedemptionRateResponse {
    pub redemption_rate: Decimal,
    pub update_time: u64,
}
