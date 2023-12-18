use std::collections::VecDeque;

use cosmwasm_schema::cw_serde;
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
pub struct LiquidStakeRate {
    /// Rate for the denom pair
    pub c_value: Decimal,
    /// Last time the rate was updated
    pub last_updated: u64,
}

impl HasTime for LiquidStakeRate {
    fn time(&self) -> u64 {
        self.last_updated
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

pub const CONFIG: Item<Config> = Item::new("config");

pub const LIQUID_STAKE_RATES: Map<&[u8], History<LiquidStakeRate>> = Map::new("liquid_stake_rate");
