use cosmwasm_std::{Deps, Order, StdError, StdResult, Uint128};

use crate::state::{
    contributions_read, pool_info_read, read_config, read_current_n, read_from_contribution_bucket,
    read_from_pool_bucket, read_pending_rewards,
};

use nebula_protocol::incentives::{
    ConfigResponse, ContributorPendingRewardsResponse, CurrentContributorInfoResponse,
    IncentivesPoolInfoResponse, PenaltyPeriodResponse, PoolType,
};

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        proxy: state.proxy.to_string(),
        custody: state.custody.to_string(),
        nebula_token: state.nebula_token.to_string(),
        owner: state.owner.to_string(),
    };

    Ok(resp)
}

/// ## Description
/// Returns the current penalty period.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_penalty_period(deps: Deps) -> StdResult<PenaltyPeriodResponse> {
    let n = read_current_n(deps.storage)?;
    let resp = PenaltyPeriodResponse { n };
    Ok(resp)
}

/// ## Description
/// Returns information of a specific pool type of a cluster address.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **pool_type** is an object of type [`u16`] which is a pool reward type.
///
/// - **cluster_address** is an object of type [`String`] which is the address
///     of a cluster contract.
///
/// - **n** is an object of type [`Option<u64>`] which is a penalty period.
pub fn query_pool_info(
    deps: Deps,
    pool_type: u16,
    cluster_address: String,
    n: Option<u64>,
) -> StdResult<IncentivesPoolInfoResponse> {
    let n = match n {
        Some(_) => n.unwrap(),
        None => read_current_n(deps.storage)?,
    };
    // Get the reward pool bucket of the given pool type
    let pool_bucket = pool_info_read(deps.storage, pool_type, n);
    // Retrieve the contract related pool info from the pool bucket
    let pool_info = read_from_pool_bucket(
        &pool_bucket,
        &deps.api.addr_validate(cluster_address.as_str())?,
    );
    let resp = IncentivesPoolInfoResponse {
        value_total: pool_info.value_total,
        reward_total: pool_info.reward_total,
    };
    Ok(resp)
}

/// ## Description
/// Returns the latest contribution of an address to a pool type of a cluster address.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **pool_type** is an object of type [`u16`] which is a pool reward type.
///
/// - **contributor_address** is an object of type [`String`] which is the address
///     of a contributor.
///
/// - **cluster_address** is an object of type [`String`] which is the address
///     of a cluster contract.
pub fn query_contributor_info(
    deps: Deps,
    pool_type: u16,
    contributor_address: String,
    cluster_address: String,
) -> StdResult<CurrentContributorInfoResponse> {
    // Get the contribution bucket based on the given pool type and address
    let contribution_bucket = contributions_read(
        deps.storage,
        &deps.api.addr_validate(contributor_address.as_str())?,
        pool_type,
    );
    // Find the contribution on a specific cluster from the contribution bucket
    let contributions = read_from_contribution_bucket(
        &contribution_bucket,
        &deps.api.addr_validate(cluster_address.as_str())?,
    );
    let resp = CurrentContributorInfoResponse {
        n: contributions.n,
        value_contributed: contributions.value_contributed,
    };
    Ok(resp)
}

/// ## Description
/// Returns the current pending rewards for an address.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **contributor_address** is an object of type [`String`] which is the address
///     of a contributor.
pub fn query_contributor_pending_rewards(
    deps: Deps,
    contributor_address: String,
) -> StdResult<ContributorPendingRewardsResponse> {
    // Validate address format
    let validated_contributor = deps.api.addr_validate(contributor_address.as_str())?;
    let mut contribution_tuples = vec![];

    // For each pool type, list all contributed clusters
    for i in PoolType::ALL_TYPES.iter() {
        let contribution_bucket = contributions_read(deps.storage, &validated_contributor, **i);
        // Get all clusters with a specific reward pool type
        for kv in contribution_bucket.range(None, None, Order::Ascending) {
            let (k, _) = kv?;

            // Validate address format of the cluster contract
            let asset_address = deps.api.addr_validate(
                std::str::from_utf8(&k)
                    .map_err(|_| StdError::generic_err("Invalid asset address"))?,
            )?;
            contribution_tuples.push((i, asset_address));
        }
    }

    // Get pending rewards that were moved to `pending_rewards` due to
    // newer contributions in the current penalty period
    let mut pending_rewards = read_pending_rewards(
        deps.storage,
        &deps.api.addr_validate(contributor_address.as_str())?,
    );

    // Get the lastest penalty period
    let n = read_current_n(deps.storage)?;
    // Get rewards from older contributions that are not moved as there
    // is no contribution in the current penalty period replacing them
    for (pool_type, asset_address) in contribution_tuples {
        let contribution_bucket =
            contributions_read(deps.storage, &validated_contributor, **pool_type);
        // Get the latest contribution on the cluster of a specific pool type
        let contribution = read_from_contribution_bucket(&contribution_bucket, &asset_address);

        // The reward from older penalty period is still here
        if contribution.value_contributed != Uint128::zero() && contribution.n != n {
            // Get the total rewards a pool type in the cluster
            let pool_bucket = pool_info_read(deps.storage, **pool_type, contribution.n);
            let pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);
            // Calculate the user pending rewards of the pool type in the cluster
            let new_pending_reward = Uint128::new(
                pool_info.reward_total.u128() * contribution.value_contributed.u128()
                    / pool_info.value_total.u128(),
            );
            pending_rewards += new_pending_reward;
        }
    }

    Ok(ContributorPendingRewardsResponse { pending_rewards })
}
