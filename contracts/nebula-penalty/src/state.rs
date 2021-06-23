use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use basket_math::FPDecimal;
use cosmwasm_std::{HumanAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};
use nebula_protocol::penalty::PenaltyParams;

/// config: BasketConfig
pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    pub owner: HumanAddr,
    pub penalty_params: PenaltyParams,

    pub ema: FPDecimal,
    pub last_block: u64,
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
