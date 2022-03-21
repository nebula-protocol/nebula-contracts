use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cluster_math::FPDecimal;
use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Singleton};
use nebula_protocol::penalty::PenaltyParams;

/// config: PenaltyConfig
pub static CONFIG_KEY: &[u8] = b"config";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing penalty contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    /// Owner of the contract, cluster contract
    pub owner: Addr,
    /// General parameters of the panalty contract
    pub penalty_params: PenaltyParams,

    /// Last rebalanced EMA
    pub ema: FPDecimal,
    /// Last rebalanced block
    pub last_block: u64,
}

pub fn config_store(storage: &mut dyn Storage) -> Singleton<PenaltyConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn store_config(storage: &mut dyn Storage, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
