use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

static KEY_CONFIG: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub distribution_contract: HumanAddr, // collected rewards receiver
    pub terraswap_factory: HumanAddr,     // terraswap factory contract
    pub nebula_token: HumanAddr,
    pub base_denom: String,
    // factory contract
    pub owner: HumanAddr,
}

pub fn store_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
