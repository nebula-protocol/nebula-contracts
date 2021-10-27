use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// prices: Map<asset:
pub static CONFIG: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub terraswap_factory: CanonicalAddr,
    pub base_denom: String,
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, CONFIG).load()
}

pub fn set_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, CONFIG).save(config)
}
