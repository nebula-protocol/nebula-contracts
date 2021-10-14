use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cluster_math::FPDecimal;
use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Singleton};
use nebula_protocol::penalty::PenaltyParams;

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    pub owner: String,
    pub penalty_params: PenaltyParams,

    pub ema: FPDecimal,
    pub last_block: u64,
}

pub fn config_store(storage: &mut dyn Storage) -> Singleton<PenaltyConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config(storage: &mut dyn Storage, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
