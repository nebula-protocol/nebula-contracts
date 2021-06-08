use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, CanonicalAddr, Extern, HumanAddr, Querier, ReadonlyStorage, StdResult, Storage, Uint128, StdError};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

static KEY_CONFIG: &[u8] = b"config";
static CURRENT_N: &[u8] = b"current_n";

static PREFIX_PENDING_REWARDS: &[u8] = b"pending_rewards";

static PREFIX_POOL_INFO: &[u8] = b"pool_info";
static PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub factory: CanonicalAddr,
    pub custody: HumanAddr,
    pub terraswap_factory: CanonicalAddr, // terraswap factory contract
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

// each pool is derived from a combination of n, pool_type, and asset_address
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    // records some value and the rewards distributed among
    // users who contributed to the value
    // your share of reward_total is proportional to your
    // share of value_total
    pub value_total: Uint128,
    pub reward_total: Uint128,
}

pub fn pool_info_store<S: Storage>(storage: &mut S, pool_type: u16, n: u64) -> Bucket<S, PoolInfo> {
    Bucket::multilevel(
        &[PREFIX_POOL_INFO, &pool_type.to_be_bytes(), &n.to_be_bytes()],
        storage,
    )
}

pub fn pool_info_read<S: Storage>(
    storage: &S,
    pool_type: u16,
    n: u64,
) -> ReadonlyBucket<S, PoolInfo> {
    ReadonlyBucket::multilevel(
        &[PREFIX_POOL_INFO, &pool_type.to_be_bytes(), &n.to_be_bytes()],
        storage,
    )
}

pub fn read_from_pool_bucket<S: Storage>(
    bucket: &ReadonlyBucket<S, PoolInfo>,
    asset_address: &CanonicalAddr,
) -> PoolInfo {
    match bucket.load(asset_address.as_slice()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolInfo {
            value_total: Uint128::zero(),
            reward_total: Uint128::zero(),
        },
    }
}

// amount of nebula each person is owed
pub fn store_pending_rewards<S: Storage>(
    storage: &mut S,
    owner: &CanonicalAddr,
    amt: Uint128,
) -> StdResult<()> {
    Bucket::new(PREFIX_PENDING_REWARDS, storage).save(owner.as_slice(), &amt)
}

pub fn read_pending_rewards<S: Storage>(storage: &S, owner: &CanonicalAddr) -> Uint128 {
    match ReadonlyBucket::new(PREFIX_PENDING_REWARDS, storage).load(owner.as_slice()) {
        Ok(pending_reward) => pending_reward,
        Err(_) => Uint128::zero()
    }
}

// each pool contribution is derived from a combination of pool_type, asset_address,
// and the owner address. the pool contribution stores the contribution of a given owner
// to some (pool_type, asset_address) pool that hasn't yet been transformed into a pending
// reward
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolContribution {
    pub n: u64,
    pub value_contributed: Uint128,
}

/// returns a bucket with all contributions from this owner (query it by owner)
pub fn contributions_store<'a, S: Storage>(
    storage: &'a mut S,
    owner: &CanonicalAddr,
    pool_type: u16,
) -> Bucket<'a, S, PoolContribution> {
    Bucket::multilevel(
        &[PREFIX_REWARD, &owner.as_slice(), &pool_type.to_be_bytes()],
        storage,
    )
}

/// returns a bucket with all contributions owned by this owner for this pool type
/// (read-only version for queries)
pub fn contributions_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
    pool_type: u16,
) -> ReadonlyBucket<'a, S, PoolContribution> {
    ReadonlyBucket::multilevel(
        &[PREFIX_REWARD, &owner.as_slice(), &pool_type.to_be_bytes()],
        storage,
    )
}

pub fn read_from_contribution_bucket<S: Storage>(
    bucket: &ReadonlyBucket<S, PoolContribution>,
    asset_address: &CanonicalAddr,
) -> PoolContribution {
    match bucket.load(asset_address.as_slice()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolContribution {
            n: 0,
            value_contributed: Uint128::zero(),
        },
    }
}

// utility functions for state

// transform contributions into pending reward
// the contribution must be from before the current n
pub fn contributions_to_pending_rewards<S: Storage>(
    storage: &mut S,
    owner_address: &CanonicalAddr,
    pool_type: u16,
    asset_address: &CanonicalAddr,
) -> StdResult<()> {
    let contribution_bucket = contributions_read(storage, &owner_address, pool_type);
    let mut contribution = read_from_contribution_bucket(&contribution_bucket, &asset_address);

    let n = read_current_n(storage)?;
    if contribution.value_contributed != Uint128::zero() && contribution.n != n {
        let pool_bucket = pool_info_read(storage, pool_type, contribution.n);
        let pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);

        // using integers here .. do we care if the remaining fractions of nebula stay in this contract?
        let new_pending_reward = read_pending_rewards(storage, &owner_address)
            + Uint128(
                pool_info.reward_total.u128() * contribution.value_contributed.u128()
                    / pool_info.value_total.u128(),
            );
        store_pending_rewards(storage, &owner_address, new_pending_reward)?;

        contribution.value_contributed = Uint128::zero();
    }
    contribution.n = n;
    contributions_store(storage, &owner_address, pool_type)
        .save(&asset_address.as_slice(), &contribution)?;
    Ok(())
}

pub fn record_contribution<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    contributor: &CanonicalAddr,
    pool_type: u16,
    asset_address: &CanonicalAddr,
    contribution_amt: Uint128,
) -> StdResult<()> {
    let n = read_current_n(&deps.storage)?;

    contributions_to_pending_rewards(&mut deps.storage, &contributor, pool_type, &asset_address)?;

    let pool_bucket = pool_info_read(&deps.storage, pool_type, n);
    let mut pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);

    let contribution_bucket = contributions_read(&deps.storage, &contributor, pool_type);
    let mut contributions = read_from_contribution_bucket(&contribution_bucket, &asset_address);

    pool_info.value_total += contribution_amt;
    contributions.value_contributed += contribution_amt;

    contributions_store(&mut deps.storage, &contributor, pool_type)
        .save(asset_address.as_slice(), &contributions)?;
    pool_info_store(&mut deps.storage, pool_type, n).save(asset_address.as_slice(), &pool_info)?;
    Ok(())
}
