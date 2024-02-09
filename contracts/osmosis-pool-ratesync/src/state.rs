use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use std::fmt;

// The config defines the admin and oracle contract addresses
#[cw_serde]
pub struct Config {
    /// The owner address is able to add and remove pools
    pub owner_address: Addr,
    /// The lsr contract address represents the address of the liquid stake rate contract
    pub lsr_contract_address: Addr,
}

/// Pool represents a stableswap pool that should have it's scaling factors adjusted
#[cw_serde]
pub struct Pool {
    /// Pool ID of the Osmosis pool (e.g. 886)
    pub pool_id: u64,
    /// The denom of the stkToken as it lives on Osmosis (e.g. ibc/{hash(transfer/channel-0/stkuatom)})
    pub stk_token_denom: String,
    /// The transfer port id
    pub transfer_port_id: String,
    /// The transfer channel id
    pub transfer_channel_id: String,
    /// The ibc hash of stkToken
    pub ibc_hash_stk_denom: String,
    /// The ordering of the stkToken vs nativeToken assets in the Osmosis pool,
    pub asset_ordering: AssetOrdering,
    /// The last time (in unix timestamp) that the scaling factors were updated
    pub last_updated: u64,
}

#[cw_serde]
pub enum AssetOrdering {
    NativeTokenFirst,
    StkTokenFirst,
}

impl fmt::Display for AssetOrdering {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetOrdering::NativeTokenFirst => write!(f, "native_token_first"),
            AssetOrdering::StkTokenFirst => write!(f, "stk_token_first"),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOLS: Map<u64, Pool> = Map::new("pools");
