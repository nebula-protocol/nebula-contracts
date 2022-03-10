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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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

    store_current_n(deps.storage, 0)?;
    Ok(Response::default())
}

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

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: &String,
) -> Result<Response, ContractError> {
    let validated_owner = deps.api.addr_validate(owner.as_str())?;
    let cfg = read_config(deps.storage)?;

    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut new_cfg = cfg;
    new_cfg.owner = validated_owner;
    store_config(deps.storage, &new_cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_owner")]))
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg = cw20_msg.msg;
    let config: Config = read_config(deps.storage)?;

    match from_binary(&msg)? {
        Cw20HookMsg::DepositReward { rewards } => {
            // only reward token contract can execute this message
            if config.nebula_token != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            let mut rewards_amount = Uint128::zero();
            for (_, _, amount) in rewards.iter() {
                rewards_amount += *amount;
            }

            if rewards_amount != cw20_msg.amount {
                return Err(ContractError::Generic(
                    "Rewards amount miss matched".to_string(),
                ));
            }

            deposit_reward(deps, rewards, cw20_msg.amount)
        }
    }
}

pub fn new_penalty_period(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;

    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    let n = read_current_n(deps.storage)?;

    let new_n = increment_n(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "new_penalty_period"),
        attr("previous_n", n.to_string()),
        attr("current_n", new_n.to_string()),
    ]))
}

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

pub fn query_penalty_period(deps: Deps) -> StdResult<PenaltyPeriodResponse> {
    let n = read_current_n(deps.storage)?;
    let resp = PenaltyPeriodResponse { n };
    Ok(resp)
}

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
    let pool_bucket = pool_info_read(deps.storage, pool_type, n);
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

pub fn query_contributor_info(
    deps: Deps,
    pool_type: u16,
    contributor_address: String,
    cluster_address: String,
) -> StdResult<CurrentContributorInfoResponse> {
    let contribution_bucket = contributions_read(
        deps.storage,
        &deps.api.addr_validate(contributor_address.as_str())?,
        pool_type,
    );
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

pub fn query_contributor_pending_rewards(
    deps: Deps,
    contributor_address: String,
) -> StdResult<ContributorPendingRewardsResponse> {
    let validated_contributor = deps.api.addr_validate(contributor_address.as_str())?;
    let mut contribution_tuples = vec![];

    for i in PoolType::ALL_TYPES.iter() {
        let contribution_bucket = contributions_read(deps.storage, &validated_contributor, **i);
        for kv in contribution_bucket.range(None, None, Order::Ascending) {
            let (k, _) = kv?;

            let asset_address = deps.api.addr_validate(
                std::str::from_utf8(&k)
                    .map_err(|_| StdError::generic_err("Invalid asset address"))?,
            )?;
            contribution_tuples.push((i, asset_address));
        }
    }

    let n = read_current_n(deps.storage)?;
    let mut pending_rewards = Uint128::zero();
    for (pool_type, asset_address) in contribution_tuples {
        let contribution_bucket =
            contributions_read(deps.storage, &validated_contributor, **pool_type);
        let contribution = read_from_contribution_bucket(&contribution_bucket, &asset_address);

        if contribution.value_contributed != Uint128::zero() && contribution.n != n {
            let pool_bucket = pool_info_read(deps.storage, **pool_type, contribution.n);
            let pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);
            let new_pending_reward = Uint128::new(
                pool_info.reward_total.u128() * contribution.value_contributed.u128()
                    / pool_info.value_total.u128(),
            );
            pending_rewards += new_pending_reward;
        }
    }

    Ok(ContributorPendingRewardsResponse { pending_rewards })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
