use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr, ReadonlyStorage, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_POOL_INFO: &[u8] = b"pool_info";

static PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
}

pub fn store_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub staking_token: HumanAddr,
    pub pending_reward: Uint128, // not distributed amount due to zero bonding
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
}

pub fn store_pool_info<S: Storage>(
    storage: &mut S,
    asset_token: &HumanAddr,
    pool_info: &PoolInfo,
) -> StdResult<()> {
    Bucket::new(PREFIX_POOL_INFO, storage).save(asset_token.as_str().as_bytes(), pool_info)
}

pub fn read_pool_info<S: Storage>(storage: &S, asset_token: &HumanAddr) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(PREFIX_POOL_INFO, storage).load(asset_token.as_str().as_bytes())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    pub index: Decimal,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a, S: Storage>(
    storage: &'a mut S,
    owner: &HumanAddr,
) -> Bucket<'a, S, RewardInfo> {
    Bucket::multilevel(&[PREFIX_REWARD, owner.as_str().as_bytes()], storage)
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &HumanAddr,
) -> ReadonlyBucket<'a, S, RewardInfo> {
    ReadonlyBucket::multilevel(&[PREFIX_REWARD, owner.as_str().as_bytes()], storage)
}