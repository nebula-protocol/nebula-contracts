use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cluster_math::FPDecimal;
use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Singleton};
use nebula_protocol::penalty::PenaltyParams;

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    pub owner: HumanAddr,
    pub penalty_params: PenaltyParams,

    pub ema: FPDecimal,
    pub last_block: u64,
}

pub fn config_store<S: Storage>(storage: &mut S) -> Singleton<S, PenaltyConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
