use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

pub static CONFIG_KEY: &[u8] = b"config";
pub static TARGET_KEY: &[u8] = b"target";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketConfig {
    pub owner: CanonicalAddr,
    pub basket_token: CanonicalAddr,
    pub oracle: CanonicalAddr,
    pub assets: Vec<CanonicalAddr>,
    pub penalty_params: PenaltyParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyParams {
    pub alpha_plus: i128,
    pub alpha_minus: i128,
    pub sigma_plus: i128,
    pub sigma_minus: i128,
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<BasketConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &BasketConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

pub fn read_target<S: Storage>(storage: &S) -> StdResult<Vec<u32>> {
    singleton_read(storage, TARGET_KEY).load()
}

pub fn save_target<S: Storage>(storage: &mut S, target: &Vec<u32>) -> StdResult<()> {
    singleton(storage, TARGET_KEY).save(target)
}
