use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{DepsMut, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

static KEY_CONFIG: &[u8] = b"config";
static CURRENT_N: &[u8] = b"current_n";

static PREFIX_PENDING_REWARDS: &[u8] = b"pending_rewards";

static PREFIX_POOL_INFO: &[u8] = b"pool_info";
static PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub factory: String,
    pub custody: String,
    pub astroport_factory: String, // astroport factory contract
    pub nebula_token: String,
    pub base_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_current_n(storage: &mut dyn Storage, n: u64) -> StdResult<()> {
    singleton(storage, CURRENT_N).save(&n)
}

pub fn read_current_n(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, CURRENT_N).load()
}

// each pool is derived from a combination of n, pool_type, and cluster_address
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    // records some value and the rewards distributed among
    // users who contributed to the value
    // your share of reward_total is proportional to your
    // share of value_total
    pub value_total: Uint128,
    pub reward_total: Uint128,
}

pub fn pool_info_store(storage: &mut dyn Storage, pool_type: u16, n: u64) -> Bucket<PoolInfo> {
    Bucket::multilevel(
        storage,
        &[PREFIX_POOL_INFO, &pool_type.to_be_bytes(), &n.to_be_bytes()],
    )
}

pub fn pool_info_read(storage: &dyn Storage, pool_type: u16, n: u64) -> ReadonlyBucket<PoolInfo> {
    ReadonlyBucket::multilevel(
        storage,
        &[PREFIX_POOL_INFO, &pool_type.to_be_bytes(), &n.to_be_bytes()],
    )
}

pub fn read_from_pool_bucket(
    bucket: &ReadonlyBucket<PoolInfo>,
    cluster_address: &String,
) -> PoolInfo {
    match bucket.load(cluster_address.as_str().as_bytes()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolInfo {
            value_total: Uint128::zero(),
            reward_total: Uint128::zero(),
        },
    }
}

// amount of nebula each person is owed
pub fn store_pending_rewards(
    storage: &mut dyn Storage,
    contributor: &String,
    amt: Uint128,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_PENDING_REWARDS).save(contributor.as_str().as_bytes(), &amt)
}

pub fn read_pending_rewards(storage: &dyn Storage, contributor: &String) -> Uint128 {
    match ReadonlyBucket::new(storage, PREFIX_PENDING_REWARDS).load(contributor.as_str().as_bytes())
    {
        Ok(pending_reward) => pending_reward,
        Err(_) => Uint128::zero(),
    }
}

// each pool contribution is derived from a combination of pool_type, cluster_address,
// and the owner address. the pool contribution stores the contribution of a given owner
// to some (pool_type, cluster_address) pool that hasn't yet been transformed into a pending
// reward
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolContribution {
    pub n: u64,
    pub value_contributed: Uint128,
}

/// returns a bucket with all contributions from this owner (query it by owner)
pub fn contributions_store<'a>(
    storage: &'a mut dyn Storage,
    contributor: &String,
    pool_type: u16,
) -> Bucket<'a, PoolContribution> {
    Bucket::multilevel(
        storage,
        &[
            PREFIX_REWARD,
            &contributor.as_str().as_bytes(),
            &pool_type.to_be_bytes(),
        ],
    )
}

/// returns a bucket with all contributions owned by this owner for this pool type
/// (read-only version for queries)
pub fn contributions_read<'a>(
    storage: &'a dyn Storage,
    contributor: &String,
    pool_type: u16,
) -> ReadonlyBucket<'a, PoolContribution> {
    ReadonlyBucket::multilevel(
        storage,
        &[
            PREFIX_REWARD,
            &contributor.as_str().as_bytes(),
            &pool_type.to_be_bytes(),
        ],
    )
}

// bucket over all cluster
pub fn read_from_contribution_bucket(
    bucket: &ReadonlyBucket<PoolContribution>,
    cluster_address: &String,
) -> PoolContribution {
    match bucket.load(cluster_address.as_str().as_bytes()) {
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
pub fn contributions_to_pending_rewards(
    storage: &mut dyn Storage,
    contributor_address: &String,
    pool_type: u16,
    cluster_address: &String,
) -> StdResult<()> {
    let contribution_bucket = contributions_read(storage, &contributor_address, pool_type);
    let mut contribution = read_from_contribution_bucket(&contribution_bucket, &cluster_address);

    let n = read_current_n(storage)?;
    if contribution.value_contributed != Uint128::zero() && contribution.n != n {
        let pool_bucket = pool_info_read(storage, pool_type, contribution.n);
        let pool_info = read_from_pool_bucket(&pool_bucket, &cluster_address);

        // using integers here .. do we care if the remaining fractions of nebula stay in this contract?
        let new_pending_reward = read_pending_rewards(storage, &contributor_address)
            + Uint128::new(
                pool_info.reward_total.u128() * contribution.value_contributed.u128()
                    / pool_info.value_total.u128(),
            );
        store_pending_rewards(storage, &contributor_address, new_pending_reward)?;

        contribution.value_contributed = Uint128::zero();
    }
    contribution.n = n;
    contributions_store(storage, &contributor_address, pool_type)
        .save(&cluster_address.as_str().as_bytes(), &contribution)?;
    Ok(())
}

pub fn record_contribution(
    deps: DepsMut,
    contributor: &String,
    pool_type: u16,
    cluster_address: &String,
    contribution_amt: Uint128,
) -> StdResult<()> {
    let n = read_current_n(deps.storage)?;

    contributions_to_pending_rewards(deps.storage, &contributor, pool_type, &cluster_address)?;

    let pool_bucket = pool_info_read(deps.storage, pool_type, n);
    let mut pool_info = read_from_pool_bucket(&pool_bucket, &cluster_address);

    let contribution_bucket = contributions_read(deps.storage, &contributor, pool_type);
    let mut contributions = read_from_contribution_bucket(&contribution_bucket, &cluster_address);

    pool_info.value_total += contribution_amt;
    contributions.value_contributed += contribution_amt;

    contributions_store(deps.storage, &contributor, pool_type)
        .save(cluster_address.as_str().as_bytes(), &contributions)?;
    pool_info_store(deps.storage, pool_type, n)
        .save(cluster_address.as_str().as_bytes(), &pool_info)?;
    Ok(())
}
