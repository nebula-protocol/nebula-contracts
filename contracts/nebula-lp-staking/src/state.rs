use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

// config: Config
pub static KEY_CONFIG: &[u8] = b"config";
// pool info: PoolInfo
pub static PREFIX_POOL_INFO: &[u8] = b"pool_info";

// reward: RewardInfo
static PREFIX_REWARD: &[u8] = b"reward";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing LP staking contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the contract
    pub owner: Addr,
    /// Nebula token contract
    pub nebula_token: Addr,
    /// Astroport factory contract
    pub astroport_factory: Addr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

//////////////////////////////////////////////////////////////////////
/// POOL INFO
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing pool information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    /// LP token contract
    pub staking_token: Addr,
    /// Not distributed amount due to zero bonding
    pub pending_reward: Uint128,
    /// Total bond in this pool
    pub total_bond_amount: Uint128,
    /// Index for reward distribution
    pub reward_index: Decimal,
}

pub fn store_pool_info(
    storage: &mut dyn Storage,
    asset_token: &Addr,
    pool_info: &PoolInfo,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_POOL_INFO).save(asset_token.as_bytes(), pool_info)
}

pub fn read_pool_info(storage: &dyn Storage, asset_token: &Addr) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(storage, PREFIX_POOL_INFO).load(asset_token.as_bytes())
}

//////////////////////////////////////////////////////////////////////
/// REWARD INFO
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing reward information in a specific pool.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    /// Current reward index for this staker
    pub index: Decimal,
    /// Bond amount of the staker
    pub bond_amount: Uint128,
    /// Pending reward of the staker
    pub pending_reward: Uint128,
}

/// Returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a>(storage: &'a mut dyn Storage, owner: &Addr) -> Bucket<'a, RewardInfo> {
    Bucket::multilevel(storage, &[PREFIX_REWARD, owner.as_bytes()])
}

/// Returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a>(storage: &'a dyn Storage, owner: &Addr) -> ReadonlyBucket<'a, RewardInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, owner.as_bytes()])
}
