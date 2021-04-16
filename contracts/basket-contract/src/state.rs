use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use basket_math::FPDecimal;
use cosmwasm_std::{HumanAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use terraswap::asset::{Asset, AssetInfo};

/// config: BasketConfig
pub static CONFIG_KEY: &[u8] = b"config";

/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";

/// asset data: Vec<AssetData>
pub static ASSET_DATA_KEY: &[u8] = b"asset_data";

/// asset: Vec<HumanAddr>
pub static ASSETS_KEY: &[u8] = b"assets";

/// staging: Map<asset: HumanAddr, Map<account: HumanAddr, staged_amount: Uint128>>
pub static PREFIX_STAGING: &[u8] = b"staging";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketConfig {
    pub name: String,
    pub owner: HumanAddr,
    pub basket_token: Option<HumanAddr>,
    pub oracle: HumanAddr,
    pub penalty_params: PenaltyParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetAssetData {
    pub asset: AssetInfo,
    pub target: u32,
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

pub fn read_target_asset_data<S: Storage>(storage: &S) -> StdResult<Vec<TargetAssetData>> {
    singleton_read(storage, ASSET_DATA_KEY).load()
}

pub fn save_target_asset_data<S: Storage>(
    storage: &mut S,
    asset_data: &Vec<TargetAssetData>,
) -> StdResult<()> {
    singleton(storage, ASSET_DATA_KEY).save(asset_data)
}

pub fn read_target<S: Storage>(storage: &S) -> StdResult<Vec<u32>> {
    singleton_read(storage, TARGET_KEY).load()
}

pub fn save_target<S: Storage>(storage: &mut S, target: &Vec<u32>) -> StdResult<()> {
    singleton(storage, TARGET_KEY).save(target)
}

// Keep record of total staged for each asset
pub fn read_total_staged_asset<S: Storage>(storage: &S, asset: &AssetInfo) -> StdResult<Uint128> {
    singleton_read(storage, &asset.to_string().as_bytes()).load()
}

pub fn save_total_staged_asset<S: Storage>(
    storage: &mut S,
    asset: &AssetInfo,
    amount: &Uint128,
) -> StdResult<()> {
    singleton(storage, &asset.to_string().as_bytes()).save(amount)
}

pub fn read_staged_asset<S: Storage>(
    storage: &S,
    account: &HumanAddr,
    asset: &AssetInfo,
) -> StdResult<Uint128> {
    let staging =
        ReadonlyBucket::multilevel(&[PREFIX_STAGING, asset.to_string().as_bytes()], storage);
    match staging.load(account.as_str().as_bytes()) {
        Ok(v) => Ok(v),
        Err(_) => Ok(Uint128::zero()),
    }
}

pub fn stage_asset<S: Storage>(
    storage: &mut S,
    account: &HumanAddr,
    asset: &AssetInfo,
    amount: Uint128,
) -> StdResult<()> {
    let curr_amount = read_staged_asset(storage, account, asset)?;

    let curr_total_staged = read_total_staged_asset(storage, asset);

    // Check if zero
    let curr_total_staged = match curr_total_staged {
        Ok(v) => v,
        Err(_) => Uint128::zero(),
    };

    // Best practice for error checking?
    save_total_staged_asset(storage, asset, &(curr_total_staged + amount))?;

    let mut staging =
        Bucket::<S, Uint128>::multilevel(&[PREFIX_STAGING, asset.to_string().as_bytes()], storage);

    staging.save(account.as_str().as_bytes(), &(curr_amount + amount))
}

pub fn unstage_asset<S: Storage>(
    storage: &mut S,
    account: &HumanAddr,
    asset: &AssetInfo,
    amount: Uint128,
) -> StdResult<()> {
    let curr_amount = read_staged_asset(storage, account, asset)?;

    let curr_total_staged = read_total_staged_asset(storage, asset)?;

    // Best practice for error checking?
    save_total_staged_asset(storage, asset, &((curr_total_staged - amount)?))?;

    let mut staging =
        Bucket::<S, Uint128>::multilevel(&[PREFIX_STAGING, asset.to_string().as_bytes()], storage);
    staging.save(account.as_str().as_bytes(), &((curr_amount - amount)?))
}
