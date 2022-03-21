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
/// This structure holds the collector contract configurations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // Collected rewards receiver (Governance contract)
    pub distribution_contract: Addr,
    // Astroport factory contract
    pub astroport_factory: Addr,
    // Nebula token contract
    pub nebula_token: Addr,
    // Base denom, UST
    pub base_denom: String,
    // Owner address, factory contract
    pub owner: Addr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}
