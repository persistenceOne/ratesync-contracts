use std::collections::VecDeque;

use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// Contract owner
    pub owner: Addr,
    /// Transfer Channel ID
    pub transfer_channel_i_d: String,
    /// Transfer Port ID
    pub transfer_port_i_d: String,
}

/// The RedemptionRate struct represents the c-value of an stkToken
#[cw_serde]
pub struct RedemptionRate {
    /// stkToken denom as an IBC hash, as it appears on the oracle chain
    pub denom: String,
    /// The c-value of the stkToken
    pub redemption_rate: Decimal,
    /// The unix timestamp representing when the c-value was last updated
    pub update_time: u64,
    /// anomaly detected
    pub anomaly_detected: bool,
}

impl HasTime for RedemptionRate {
    fn time(&self) -> u64 {
        self.update_time
    }
}

pub trait HasTime {
    fn time(&self) -> u64;
}

#[cw_serde]
pub struct History<T: HasTime + Clone> {
    deque: VecDeque<T>,
    capacity: u64,
}

const HISTORY_ITEM_CAP: u64 = 100;

impl<T: HasTime + Clone> Default for History<T> {
    fn default() -> Self {
        Self::new(HISTORY_ITEM_CAP)
    }
}

impl<T: HasTime + Clone> History<T> {
    pub fn new(capacity: u64) -> Self {
        History {
            deque: VecDeque::with_capacity(capacity as usize),
            capacity,
        }
    }

    pub fn add(&mut self, item: T) {
        match self.deque.binary_search_by_key(&item.time(), |m| m.time()) {
            Ok(index) => {
                self.deque.remove(index);
                self.deque.insert(index, item)
            }
            Err(index) => {
                self.deque.insert(index, item);
                if self.deque.len() > self.capacity as usize {
                    self.deque.pop_front();
                }
            }
        }
    }

    pub fn get_latest(&self) -> Option<T> {
        self.deque.back().cloned()
    }

    pub fn get_latest_range(&self, n: usize) -> Vec<T> {
        self.deque.iter().rev().take(n).cloned().collect()
    }

    pub fn get_all(&self) -> Vec<T> {
        self.deque.iter().rev().cloned().collect()
    }
}

#[cw_serde]
pub struct AnomalyConfig {
    /// Number of last rates to consider
    pub count_limit: u64,
    /// Allowed anomaly threshold
    pub threshold: Decimal,
}

const ANOMALY_THRESHOLD: Decimal = Decimal::percent(5);

impl Default for AnomalyConfig {
    fn default() -> Self {
        AnomalyConfig {
            count_limit: HISTORY_ITEM_CAP,
            threshold: ANOMALY_THRESHOLD,
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const LIQUID_STAKE_RATES: Map<&str, History<RedemptionRate>> = Map::new("liquid_stake_rate");

pub const ANOMALY_CONFIG_BY_DENOM: Map<&str, AnomalyConfig> = Map::new("anomaly_config_by_denom");
