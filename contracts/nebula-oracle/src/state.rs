use std::u64;

use cosmwasm_std::{Decimal, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_PRICES: &[u8] = b"prices";
pub static KEY_TIMESTAMP: &[u8] = b"timestamps";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceInfo {
    pub price: Decimal,
    pub last_updated_time: u64,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_price(storage: &mut dyn Storage, asset: &String, price: &PriceInfo) -> StdResult<()> {
    bucket(storage, PREFIX_PRICES).save(asset.as_bytes(), price)
}

pub fn read_price(storage: &dyn Storage, asset: &String) -> StdResult<PriceInfo> {
    bucket_read(storage, PREFIX_PRICES).load(asset.as_bytes())
}
