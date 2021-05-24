use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdResult, Storage, Uint128, HumanAddr};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

static KEY_CONFIG: &[u8] = b"config";
static CURRENT_N: &[u8] = b"current_n";
static PREFIX_POOL_INFO: &[u8] = b"pool_info";
static PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub factory: CanonicalAddr, // collected rewards receiver
    pub terraswap_factory: CanonicalAddr,     // terraswap factory contract
    pub nebula_token: CanonicalAddr,
    pub base_denom: String,
    pub owner: HumanAddr,
}

pub fn store_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_current_n<S: Storage>(storage: &mut S, n: u64) -> StdResult<()> {
    singleton(storage, CURRENT_N).save(&n)
}

pub fn read_current_n<S: Storage>(storage: &S) -> StdResult<u64> {
    singleton_read(storage, CURRENT_N).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub penalty_sum: Uint128,
    pub reward_sum: Uint128,
}

pub fn pool_info_store<S: Storage>(storage: &mut S, n: u64) -> Bucket<S, PoolInfo> {
    Bucket::multilevel(&[PREFIX_POOL_INFO, &n.to_be_bytes()], storage)
}

pub fn pool_info_read<S: Storage>(storage: &S, n: u64) -> ReadonlyBucket<S, PoolInfo> {
    ReadonlyBucket::multilevel(&[PREFIX_POOL_INFO, &n.to_be_bytes()], storage)
}

pub fn read_from_pool_bucket<S: Storage>(
    bucket: &ReadonlyBucket<S, PoolInfo>,
    asset_address: &CanonicalAddr,
) -> PoolInfo {
    match bucket.load(asset_address.as_slice()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolInfo {
            penalty_sum: Uint128::zero(),
            reward_sum: Uint128::zero(),
        },
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    pub n: u64,
    pub penalty: Uint128,
    pub pending_reward: Uint128,
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a, S: Storage>(
    storage: &'a mut S,
    owner: &CanonicalAddr,
) -> Bucket<'a, S, RewardInfo> {
    Bucket::multilevel(&[PREFIX_REWARD, &owner.as_slice()], storage)
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, S, RewardInfo> {
    ReadonlyBucket::multilevel(&[PREFIX_REWARD, &owner.as_slice()], storage)
}

pub fn read_from_reward_bucket<S: Storage>(
    bucket: &ReadonlyBucket<S, RewardInfo>,
    asset_address: &CanonicalAddr,
) -> RewardInfo {
    match bucket.load(asset_address.as_slice()) {
        Ok(reward_info) => reward_info,
        Err(_) => RewardInfo {
            n: 0,
            penalty: Uint128::zero(),
            pending_reward: Uint128::zero()
        },
    }
}