use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

static KEY_CONFIG: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub distribution_contract: String, // collected rewards receiver
    pub terraswap_factory: String,     // terraswap factory contract
    pub nebula_token: String,
    pub base_denom: String,
    // factory contract
    pub owner: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
