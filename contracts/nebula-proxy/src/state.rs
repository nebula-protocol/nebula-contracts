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
/// A custom struct for storing the proxy contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the contract
    pub owner: Addr,
    /// Cluster factory contract
    pub factory: Addr,
    /// Incentives contract
    pub incentives: Option<Addr>,
    /// Astroport factory contract
    pub astroport_factory: Addr,
    /// Nebula token contract
    pub nebula_token: Addr,
    /// Base denom, UST
    pub base_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
