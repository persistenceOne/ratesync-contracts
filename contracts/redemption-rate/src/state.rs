use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// New contract owner, temporary holding value
    pub new_owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct RedemptionRate {
    /// Redemption Rate for the denom pair
    pub exchange_rate: Decimal,
    /// Last time the redemption rate was updated
    pub last_updated: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PRICES: Map<&[u8], RedemptionRate> = Map::new("redemption_rate");
