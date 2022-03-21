use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

/// config: Config
static KEY_CONFIG: &[u8] = b"config";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// This structure holds the community contract configurations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // Owner address, Nebula Governance contract
    pub owner: Addr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
