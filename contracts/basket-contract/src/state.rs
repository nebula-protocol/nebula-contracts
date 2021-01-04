use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use basket_math::FPDecimal;
use cosmwasm_std::{HumanAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

/// config: BasketConfig
pub static CONFIG_KEY: &[u8] = b"config";

/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";

/// staging: Map<asset: HumanAddr, Map<account: HumanAddr, staged_amount: Uint128>>
pub static PREFIX_STAGING: &[u8] = b"staging";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketConfig {
    pub name: String,
    pub owner: HumanAddr,
    pub basket_token: Option<HumanAddr>,
    pub oracle: HumanAddr,
    pub assets: Vec<HumanAddr>,
    pub penalty_params: PenaltyParams,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyParams {
    pub a_pos: FPDecimal,
    pub s_pos: FPDecimal,
    pub a_neg: FPDecimal,
    pub s_neg: FPDecimal,
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

pub fn read_staged_asset<S: Storage>(
    storage: &S,
    account: &HumanAddr,
    asset: &HumanAddr,
) -> StdResult<Uint128> {
    let staging = ReadonlyBucket::multilevel(&[PREFIX_STAGING, asset.as_str().as_bytes()], storage);
    match staging.load(account.as_str().as_bytes()) {
        Ok(v) => Ok(v),
        Err(_) => Ok(Uint128::zero()),
    }
}

pub fn stage_asset<S: Storage>(
    storage: &mut S,
    account: &HumanAddr,
    asset: &HumanAddr,
    amount: Uint128,
) -> StdResult<()> {
    let curr_amount = read_staged_asset(storage, account, asset)?;
    let mut staging =
        Bucket::<S, Uint128>::multilevel(&[PREFIX_STAGING, asset.as_str().as_bytes()], storage);
    staging.save(account.as_str().as_bytes(), &(curr_amount + amount))
}

pub fn unstage_asset<S: Storage>(
    storage: &mut S,
    account: &HumanAddr,
    asset: &HumanAddr,
    amount: Uint128,
) -> StdResult<()> {
    let curr_amount = read_staged_asset(storage, account, asset)?;
    let mut staging =
        Bucket::<S, Uint128>::multilevel(&[PREFIX_STAGING, asset.as_str().as_bytes()], storage);
    staging.save(account.as_str().as_bytes(), &((curr_amount - amount)?))
}
