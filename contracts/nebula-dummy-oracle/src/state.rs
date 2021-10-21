use std::u64;

use cosmwasm_std::{Decimal, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_PRICES: &[u8] = b"prices";
pub static KEY_TIMESTAMP: &[u8] = b"timestamps";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn config_store(storage: &mut dyn Storage) -> Singleton<Config> {
    singleton(storage, KEY_CONFIG)
}

pub fn read_price(storage: &dyn Storage, asset: &String) -> StdResult<Decimal> {
    bucket_read(storage, PREFIX_PRICES).load(asset.as_bytes())
}

pub fn set_price(storage: &mut dyn Storage, asset: &String, price: &Decimal) -> StdResult<()> {
    bucket(storage, PREFIX_PRICES).save(asset.as_bytes(), price)
}

pub fn store_last_update_time(storage: &mut dyn Storage, timestamp: &u64) -> StdResult<()> {
    singleton(storage, KEY_TIMESTAMP).save(timestamp)
}

pub fn read_last_update_time(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_TIMESTAMP).load()
}
