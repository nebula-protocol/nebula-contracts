#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, Uint128,
};

use crate::arbitrageurs::{
    arb_cluster_create, arb_cluster_redeem, record_astroport_impact, send_all, swap_all,
};
use crate::error::ContractError;
use crate::rebalancers::{
    create, internal_rewarded_create, internal_rewarded_redeem, record_rebalancer_rewards, redeem,
};
use crate::rewards::{deposit_reward, increment_n, withdraw_reward};
use crate::state::{
    contributions_read, pool_info_read, read_config, read_current_n, read_from_contribution_bucket,
    read_from_pool_bucket, store_config, store_current_n, Config,
};
use cw20::Cw20ReceiveMsg;
use nebula_protocol::incentives::{
    ConfigResponse, ContributorPendingRewardsResponse, CurrentContributorInfoResponse, Cw20HookMsg,
    ExecuteMsg, IncentivesPoolInfoResponse, InstantiateMsg, MigrateMsg, PenaltyPeriodResponse,
    PoolType, QueryMsg,
};

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Set the initial contract settings
    store_config(
        deps.storage,
        &Config {
            factory: deps.api.addr_validate(msg.factory.as_str())?,
            custody: deps.api.addr_validate(msg.custody.as_str())?,
            astroport_factory: deps.api.addr_validate(msg.astroport_factory.as_str())?,
            nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
            base_denom: msg.base_denom,
            owner: deps.api.addr_validate(msg.owner.as_str())?,
        },
    )?;
    // Set the current penalty period to be 0
    store_current_n(deps.storage, 0)?;
    Ok(Response::default())
}

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateOwner {
///             owner,
///         }** Updates the contract owner.
///
/// - **ExecuteMsg::Receive (msg)** Receives CW20 tokens and executes a hook message.
///
/// - **ExecuteMsg::Withdraw {}** Withdraws rewards for the sender.
///
/// - **ExecuteMsg::NewPenaltyPeriod {} Increments the penalty period by one.
///
/// - **ExecuteMsg::_SwapAll {
///             astroport_pair,
///             cluster_token,
///             to_ust,
///             min_return,
///         }** Swaps either all UST to the specified cluster token or vice versa.
///
/// - **ExecuteMsg::_SendAll {
///             asset_infos,
///             send_to,
///          }** Sends all specifed assets to the provided receiver.
///
/// - **ExecuteMsg::_RecordAstroportImpact {
///             arbitrageur,
///             astroport_pair,
///             cluster_contract,
///             pool_before,
///         }** Records arbitrage contribution for the reward distribution.
///
///  - **ExecuteMsg::_RecordRebalancerRewards {
///             rebalancer,
///             cluster_contract,
///             original_imbalance,
///         }** Records rebalance contribution for the reward distribution.
///
/// - **ExecuteMsg::_InternalRewardedCreate {
///             rebalancer,
///             cluster_contract,
///             asset_amounts,
///             min_tokens,
///         }** Calls the actual create logic in a cluster contract used in both arbitraging and rebalancing.
///
/// - **ExecuteMsg::_InternalRewardedRedeem {
///             rebalancer,
///             cluster_contract,
///             cluster_token,
///             max_tokens,
///             asset_amounts,
///         }** Calls the actual redeem logic in a cluster contract used in both arbitraging and rebalancing.
///
/// - **ExecuteMsg::ArbClusterCreate {
///             cluster_contract,
///             assets,
///             min_ust,
///         } ** Executes the create operation and uses CT to arbitrage on Astroport.
///
/// - **ExecuteMsg::ArbClusterRedeem {
///             cluster_contract,
///             asset,
///             min_cluster,
///         }** Executes arbitrage on Astroport to get CT and perform the redeem operation.
///
/// - **ExecuteMsg::IncentivesCreate {
///             cluster_contract,
///             asset_amounts,
///             min_tokens,
///         }** Executes the create operation on a specific cluster.
///
/// - **ExecuteMsg::IncentivesRedeem {
///             cluster_contract,
///             max_tokens,
///             asset_amounts,
///         }** Executes the redeem operation on a specific cluster.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, &owner),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::Withdraw {} => withdraw_reward(deps, info),
        ExecuteMsg::NewPenaltyPeriod {} => new_penalty_period(deps, info),
        ExecuteMsg::_SwapAll {
            astroport_pair,
            cluster_token,
            to_ust,
            min_return,
        } => swap_all(
            deps,
            env,
            info,
            astroport_pair,
            cluster_token,
            to_ust,
            min_return,
        ),
        ExecuteMsg::_SendAll {
            asset_infos,
            send_to,
        } => send_all(deps, env, info, &asset_infos, send_to),
        ExecuteMsg::_RecordAstroportImpact {
            arbitrageur,
            astroport_pair,
            cluster_contract,
            pool_before,
        } => record_astroport_impact(
            deps,
            env,
            info,
            arbitrageur,
            astroport_pair,
            cluster_contract,
            pool_before,
        ),
        ExecuteMsg::_RecordRebalancerRewards {
            rebalancer,
            cluster_contract,
            original_imbalance,
        } => record_rebalancer_rewards(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            original_imbalance,
        ),
        ExecuteMsg::_InternalRewardedCreate {
            rebalancer,
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => internal_rewarded_create(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            &asset_amounts,
            min_tokens,
        ),
        ExecuteMsg::_InternalRewardedRedeem {
            rebalancer,
            cluster_contract,
            cluster_token,
            max_tokens,
            asset_amounts,
        } => internal_rewarded_redeem(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            cluster_token,
            max_tokens,
            asset_amounts,
        ),
        ExecuteMsg::ArbClusterCreate {
            cluster_contract,
            assets,
            min_ust,
        } => arb_cluster_create(deps, env, info, cluster_contract, &assets, min_ust),
        ExecuteMsg::ArbClusterRedeem {
            cluster_contract,
            asset,
            min_cluster,
        } => arb_cluster_redeem(deps, env, info, cluster_contract, asset, min_cluster),
        ExecuteMsg::IncentivesCreate {
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => create(
            deps,
            env,
            info,
            cluster_contract,
            &asset_amounts,
            min_tokens,
        ),
        ExecuteMsg::IncentivesRedeem {
            cluster_contract,
            max_tokens,
            asset_amounts,
        } => redeem(deps, env, info, cluster_contract, max_tokens, asset_amounts),
    }
}

/// ## Description
/// Updates the owner of the incentives contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is a reference to an object of type [`String`].
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: &String,
) -> Result<Response, ContractError> {
    // Validate the address
    let validated_owner = deps.api.addr_validate(owner.as_str())?;
    let cfg = read_config(deps.storage)?;

    // Permission check
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Change owner and save
    let mut new_cfg = cfg;
    new_cfg.owner = validated_owner;
    store_config(deps.storage, &new_cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_owner")]))
}

/// ## Description
/// Receives CW20 tokens and executes a hook message.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cw20_msg** is an object of type [`Cw20ReceiveMsg`] which is a hook message to be executed.
pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg = cw20_msg.msg;
    let config: Config = read_config(deps.storage)?;

    match from_binary(&msg)? {
        Cw20HookMsg::DepositReward { rewards } => {
            // Permission check - need to only be sent from Nebula token contract
            if config.nebula_token != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            let mut rewards_amount = Uint128::zero();
            for (_, _, amount) in rewards.iter() {
                rewards_amount += *amount;
            }
            // Validate the transferred amount
            if rewards_amount != cw20_msg.amount {
                return Err(ContractError::Generic(
                    "Rewards amount miss matched".to_string(),
                ));
            }
            // Add rewards to their pools -- rebalance or arbitrage
            deposit_reward(deps, rewards, cw20_msg.amount)
        }
    }
}

/// ## Description
/// Increments the penalty period by one.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// ## Executor
/// Only the owner can execute this.
pub fn new_penalty_period(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;

    // Permission check
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Get the current penalty period
    let n = read_current_n(deps.storage)?;
    // Increases the penalty period by 1
    let new_n = increment_n(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "new_penalty_period"),
        attr("previous_n", n.to_string()),
        attr("current_n", new_n.to_string()),
    ]))
}

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Config {}** Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// - **QueryMsg::PenaltyPeriod {}** Returns the current penalty period.
///
/// - **QueryMsg::PoolInfo {
///             pool_type,
///             cluster_address,
///             n,
///         }** Returns information of a specific pool type of a cluster address.
///
/// - **QueryMsg::CurrentContributorInfo {
///             pool_type,
///             contributor_address,
///             cluster_address,
///         }** Returns the latest contribution of an address to a pool type of a cluster address.
///
/// - **QueryMsg::ContributorPendingRewards {
///             contributor_address,
///         }** Returns the current pending rewards for an address.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PenaltyPeriod {} => to_binary(&query_penalty_period(deps)?),
        QueryMsg::PoolInfo {
            pool_type,
            cluster_address,
            n,
        } => to_binary(&query_pool_info(deps, pool_type, cluster_address, n)?),
        QueryMsg::CurrentContributorInfo {
            pool_type,
            contributor_address,
            cluster_address,
        } => to_binary(&query_contributor_info(
            deps,
            pool_type,
            contributor_address,
            cluster_address,
        )?),
        QueryMsg::ContributorPendingRewards {
            contributor_address,
        } => to_binary(&query_contributor_pending_rewards(
            deps,
            contributor_address,
        )?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        factory: state.factory.to_string(),
        custody: state.custody.to_string(),
        astroport_factory: state.astroport_factory.to_string(),
        nebula_token: state.nebula_token.to_string(),
        base_denom: state.base_denom,
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

    // Get the lastest penalty period
    let n = read_current_n(deps.storage)?;
    let mut pending_rewards = Uint128::zero();
    for (pool_type, asset_address) in contribution_tuples {
        let contribution_bucket =
            contributions_read(deps.storage, &validated_contributor, **pool_type);
        // Get the latest contribution on the cluster of a specific pool type
        let contribution = read_from_contribution_bucket(&contribution_bucket, &asset_address);

        // The reward is not already claimed, and the contribution is in the lastest period
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

/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
