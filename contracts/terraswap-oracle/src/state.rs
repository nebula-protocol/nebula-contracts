use cosmwasm_std::{StdResult, Storage, HumanAddr};
use cosmwasm_storage::{singleton_read, singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// prices: Map<asset:
pub static CONFIG: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub terraswap_factory: HumanAddr,
    pub base_denom: String,
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, CONFIG).load()
}

pub fn set_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, CONFIG).save(config)
}
