use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// config: Config
pub static KEY_CONFIG: &[u8] = b"config";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing oracle contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the contract
    pub owner: Addr,
    /// TeFi oracle hub contract
    pub oracle_addr: Addr,
    /// Base denom, UST
    pub base_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
