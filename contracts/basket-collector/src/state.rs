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
    pub distribution_contract: CanonicalAddr, // collected rewards receiver
    pub terraswap_factory: CanonicalAddr,     // terraswap factory contract
    pub nebula_token: CanonicalAddr,
    pub base_denom: String,
    // factory contract
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
    pub n: u64,
    pub penalty_sum: Uint128,
    pub reward_sum: Uint128,
}

pub fn store_pool_info<S: Storage>(storage: &mut S, n: u64, pool_info: &PoolInfo) -> StdResult<()> {
    Bucket::new(PREFIX_POOL_INFO, storage).save(&n.to_be_bytes(), pool_info)
}

pub fn read_pool_info<S: Storage>(storage: &S, n: u64) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(PREFIX_POOL_INFO, storage).load(&n.to_be_bytes())
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
    reward_info: &RewardInfo,
) -> StdResult<()> {
    Bucket::new(PREFIX_REWARD, storage).save(owner.as_slice(), reward_info)
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
) -> StdResult<RewardInfo> {
    match ReadonlyBucket::new(PREFIX_REWARD, storage).load(owner.as_slice()) {
        Ok(reward_info) => Ok(reward_info),
        Err(_) => Ok(RewardInfo {
            n: 0,
            penalty: Uint128::zero(),
            pending_reward: Uint128::zero(),
        }),
    }
}
