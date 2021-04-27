use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use basket_math::FPDecimal;
use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

/// config: BasketConfig
pub static CONFIG_KEY: &[u8] = b"config";


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    pub penalty_params: PenaltyParams,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyParams {
    pub a_pos: FPDecimal,
    pub s_pos: FPDecimal,
    pub a_neg: FPDecimal,
    pub s_neg: FPDecimal,
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
