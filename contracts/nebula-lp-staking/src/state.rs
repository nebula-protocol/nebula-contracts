use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_POOL_INFO: &[u8] = b"pool_info";

static PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub nebula_token: CanonicalAddr,
    pub terraswap_factory: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub staking_token: CanonicalAddr,
    pub pending_reward: Uint128, // not distributed amount due to zero bonding
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
}

pub fn store_pool_info(
    storage: &mut dyn Storage,
    asset_token: &CanonicalAddr,
    pool_info: &PoolInfo,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_POOL_INFO).save(asset_token.as_slice(), pool_info)
}

pub fn read_pool_info(storage: &dyn Storage, asset_token: &CanonicalAddr) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(storage, PREFIX_POOL_INFO).load(asset_token.as_slice())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    pub index: Decimal,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a>(
    storage: &'a mut dyn Storage,
    owner: &CanonicalAddr,
) -> Bucket<'a, RewardInfo> {
    Bucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a>(
    storage: &'a dyn Storage,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, RewardInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
}
