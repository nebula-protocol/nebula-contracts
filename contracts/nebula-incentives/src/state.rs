use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, DepsMut, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

/// config: Config
static KEY_CONFIG: &[u8] = b"config";
/// current penalty period: u64
static CURRENT_N: &[u8] = b"current_n";

/// pending rewards: Uint128
static PREFIX_PENDING_REWARDS: &[u8] = b"pending_rewards";

/// pool info: Bucket<pool_type, penalty_period>; cluster_addr -> PoolInfo
static PREFIX_POOL_INFO: &[u8] = b"pool_info";
/// reward: Bucket<contributor, pool_type>; cluster_addr -> PoolContribution
static PREFIX_REWARD: &[u8] = b"reward";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing the incentives contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the contract
    pub owner: Addr,
    /// Cluster factory contract
    pub factory: Addr,
    /// Custody contract
    pub custody: Addr,
    /// Astroport factory contract
    pub astroport_factory: Addr,
    /// Nebula token contract
    pub nebula_token: Addr,
    /// Base denom, UST
    pub base_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

//////////////////////////////////////////////////////////////////////
/// PENALTY PERIOD
//////////////////////////////////////////////////////////////////////

pub fn store_current_n(storage: &mut dyn Storage, n: u64) -> StdResult<()> {
    singleton(storage, CURRENT_N).save(&n)
}

pub fn read_current_n(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, CURRENT_N).load()
}

//////////////////////////////////////////////////////////////////////
/// PENDING REWARDS
//////////////////////////////////////////////////////////////////////

pub fn store_pending_rewards(
    storage: &mut dyn Storage,
    contributor: &Addr,
    amt: Uint128,
) -> StdResult<()> {
    // Amount of Nebula each person is owed
    Bucket::new(storage, PREFIX_PENDING_REWARDS).save(contributor.as_bytes(), &amt)
}

pub fn read_pending_rewards(storage: &dyn Storage, contributor: &Addr) -> Uint128 {
    match ReadonlyBucket::new(storage, PREFIX_PENDING_REWARDS).load(contributor.as_bytes()) {
        Ok(pending_reward) => pending_reward,
        Err(_) => Uint128::zero(),
    }
}

//////////////////////////////////////////////////////////////////////
/// POOL INFO (bucket multilevel)
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for recording the total contribution value and rewards.
/// Each user share of `reward_total` is proportional to their share of value_total.
///
/// Each `PoolInfo` is derived from a combination of n, pool_type, and cluster_address.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    /// Total contributions among users
    pub value_total: Uint128,
    /// Total rewards to be distributed
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
    cluster_address: &Addr,
) -> PoolInfo {
    match bucket.load(cluster_address.as_bytes()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolInfo {
            value_total: Uint128::zero(),
            reward_total: Uint128::zero(),
        },
    }
}

//////////////////////////////////////////////////////////////////////
/// POOL REWARD (bucket multilevel)
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing the contribution of a given address to some
/// (pool_type, cluster_address) pool that hasn't yet been transformed into a pending
/// reward
///
/// Each `PoolContribution` is derived from a combination of pool_type, cluster_address,
/// and the owner address.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolContribution {
    /// Penalty period of this pool latest contribution
    pub n: u64,
    /// How much a user has contributed to this pool
    pub value_contributed: Uint128,
}

/// ## Description
/// Returns a bucket with all contributions owned by an address.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **contributor** is a reference to an object of type [`Addr`] which is the
///     address of a contributor.
///
/// - **pool_type** is an object of type [`u16`] which is the type of pool rewards.
pub fn contributions_store<'a>(
    storage: &'a mut dyn Storage,
    contributor: &Addr,
    pool_type: u16,
) -> Bucket<'a, PoolContribution> {
    Bucket::multilevel(
        storage,
        &[
            PREFIX_REWARD,
            &contributor.as_bytes(),
            &pool_type.to_be_bytes(),
        ],
    )
}

/// ## Description
/// Returns a bucket with all contributions owned by an address.
/// (read-only version for queries)
///
/// ## Params
/// - **storage** is a reference to an object implementing trait [`Storage`].
///
/// - **contributor** is a reference to an object of type [`Addr`] which is the
///     address of a contributor.
///
/// - **pool_type** is an object of type [`u16`] which is the type of pool rewards.
pub fn contributions_read<'a>(
    storage: &'a dyn Storage,
    contributor: &Addr,
    pool_type: u16,
) -> ReadonlyBucket<'a, PoolContribution> {
    ReadonlyBucket::multilevel(
        storage,
        &[
            PREFIX_REWARD,
            &contributor.as_bytes(),
            &pool_type.to_be_bytes(),
        ],
    )
}

/// ## Description
/// Returns `PoolContribution` of a specific cluster from a bucket over all cluster.
///
/// ## Params
/// - **bucket** is a reference to an object of type [`ReadonlyBucket<PoolContribution>`]
///     which is a bucket corresponding to the contributor and pool type.
///
/// - **cluster_address** is a reference to an object of type [`Addr`] which is the
///     address of a cluster contract.
pub fn read_from_contribution_bucket(
    bucket: &ReadonlyBucket<PoolContribution>,
    cluster_address: &Addr,
) -> PoolContribution {
    match bucket.load(cluster_address.as_bytes()) {
        Ok(reward_info) => reward_info,
        Err(_) => PoolContribution {
            n: 0,
            value_contributed: Uint128::zero(),
        },
    }
}

//////////////////////////////////////////////////////////////////////
/// UTILS FOR STATE
//////////////////////////////////////////////////////////////////////

/// ## Description
/// Transform contributions into pending reward.
/// The contribution must be from before the current penalty period (n).
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **contributor_address** is a reference to an object of type [`Addr`] which is the
///     address of the contributor.
///
/// - **pool_type** is an object of type [`u16`] which is the type of the reward pool.
///
/// - **cluster_address** is a reference to an object of type [`Addr`] which is the
///     address of a cluster contract.
pub fn contributions_to_pending_rewards(
    storage: &mut dyn Storage,
    contributor_address: &Addr,
    pool_type: u16,
    cluster_address: &Addr,
) -> StdResult<()> {
    // Retrieve a `PoolContribution` corresponding to the contributor, pool type, and the cluster
    let contribution_bucket = contributions_read(storage, &contributor_address, pool_type);
    let mut contribution = read_from_contribution_bucket(&contribution_bucket, cluster_address);

    // Get the current penalty period
    let n = read_current_n(storage)?;
    if contribution.value_contributed != Uint128::zero() && contribution.n != n {
        let pool_bucket = pool_info_read(storage, pool_type, contribution.n);
        let pool_info = read_from_pool_bucket(&pool_bucket, cluster_address);

        // using integers here .. do we care if the remaining fractions of nebula stay in this contract?
        let new_pending_reward = read_pending_rewards(storage, &contributor_address)
            + Uint128::new(
                pool_info.reward_total.u128() * contribution.value_contributed.u128()
                    / pool_info.value_total.u128(),
            );
        store_pending_rewards(storage, &contributor_address, new_pending_reward)?;

        contribution.value_contributed = Uint128::zero();
    }
    // Update penalty period of the contribution
    contribution.n = n;
    contributions_store(storage, &contributor_address, pool_type)
        .save(cluster_address.as_bytes(), &contribution)?;
    Ok(())
}

/// ## Description
/// Records the either the rebalance or arbitrage contributions of a user.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **contributor** is a reference to an object of type [`Addr`] which is the
///     address of the contributor.
///
/// - **pool_type** is an object of type [`u16`] which is the type of the reward pool.
///
/// - **cluster_address** is a reference to an object of type [`Addr`] which is the
///     address of a cluster contract.
///
/// - **contribution_amt** is an object of type [`Uint128`] which is the contribution on
pub fn record_contribution(
    deps: DepsMut,
    contributor: &Addr,
    pool_type: u16,
    cluster_address: &Addr,
    contribution_amt: Uint128,
) -> StdResult<()> {
    let n = read_current_n(deps.storage)?;

    // Convert contributions in old penalty period to rewards,
    // and update the contribution penalty period
    contributions_to_pending_rewards(deps.storage, &contributor, pool_type, cluster_address)?;

    // Get `PoolInfo` corresponding to the pool type, penalty period (n), and cluster address
    let pool_bucket = pool_info_read(deps.storage, pool_type, n);
    let mut pool_info = read_from_pool_bucket(&pool_bucket, cluster_address);

    // Get `PoolContribution` corresponding to the contributor, pool type, and cluster address
    let contribution_bucket = contributions_read(deps.storage, &contributor, pool_type);
    let mut contributions = read_from_contribution_bucket(&contribution_bucket, cluster_address);

    // Increase the total contribution of the pool
    pool_info.value_total += contribution_amt;
    // Increase the user contribution to the pool
    contributions.value_contributed += contribution_amt;

    contributions_store(deps.storage, &contributor, pool_type)
        .save(cluster_address.as_bytes(), &contributions)?;
    pool_info_store(deps.storage, pool_type, n).save(cluster_address.as_bytes(), &pool_info)?;
    Ok(())
}
