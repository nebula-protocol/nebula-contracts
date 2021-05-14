use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use basket_math::FPDecimal;
use cosmwasm_std::{StdResult, Storage, HumanAddr};
use cosmwasm_storage::{singleton, singleton_read};

/// config: BasketConfig
pub static CONFIG_KEY: &[u8] = b"config";


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyConfig {
    pub owner: HumanAddr,
    pub penalty_params: PenaltyParams,

    pub ema: FPDecimal,
    pub last_block: u64,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyParams {
    // penalty_amt_lo -> amount of penalty when imbalance <= penalty_cutoff_lo * E
    pub penalty_amt_lo: FPDecimal,
    pub penalty_cutoff_lo: FPDecimal,

    // penalty_amt_hi -> amount of penalty when imbalance >= penalty_cutoff_hi * E
    pub penalty_amt_hi: FPDecimal,
    pub penalty_cutoff_hi: FPDecimal,
    // in between penalty_cutoff_hi and penalty_cutoff_lo, the amount of penalty increases linearly

    // reward_amt -> amount of reward when imbalance >= reward_cutoff * E
    // no reward everywhere else
    pub reward_amt: FPDecimal,
    pub reward_cutoff: FPDecimal,
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<PenaltyConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &PenaltyConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}
