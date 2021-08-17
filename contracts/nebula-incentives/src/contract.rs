use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use crate::arbitrageurs::{
    arb_cluster_mint, arb_cluster_redeem, record_terraswap_impact, send_all, swap_all,
};
use crate::rebalancers::{
    internal_rewarded_mint, internal_rewarded_redeem, mint, record_rebalancer_rewards, redeem,
};
use crate::rewards::{deposit_reward, increment_n, withdraw_reward};
use crate::state::{
    contributions_read, pool_info_read, read_config, read_current_n, read_from_contribution_bucket,
    read_from_pool_bucket, read_pending_rewards, store_config, store_current_n, Config,
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
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            factory: msg.factory,
            custody: msg.custody,
            terraswap_factory: msg.terraswap_factory,
            nebula_token: msg.nebula_token,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    store_current_n(deps.storage, 0)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, &owner),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::Withdraw {} => withdraw_reward(deps, info),
        ExecuteMsg::NewPenaltyPeriod {} => new_penalty_period(deps, info),
        ExecuteMsg::_SwapAll {
            terraswap_pair,
            cluster_token,
            to_ust,
            min_return,
        } => swap_all(
            deps,
            env,
            info,
            terraswap_pair,
            cluster_token,
            to_ust,
            min_return,
        ),
        ExecuteMsg::_SendAll {
            asset_infos,
            send_to,
        } => send_all(deps, env, info, &asset_infos, send_to),
        ExecuteMsg::_RecordTerraswapImpact {
            arbitrageur,
            terraswap_pair,
            cluster_contract,
            pool_before,
        } => record_terraswap_impact(
            deps,
            env,
            info,
            arbitrageur,
            terraswap_pair,
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
        ExecuteMsg::_InternalRewardedMint {
            rebalancer,
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => internal_rewarded_mint(
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
        ExecuteMsg::ArbClusterMint {
            cluster_contract,
            assets,
            min_ust,
        } => arb_cluster_mint(deps, env, info, cluster_contract, &assets, min_ust),
        ExecuteMsg::ArbClusterRedeem {
            cluster_contract,
            asset,
            min_cluster,
        } => arb_cluster_redeem(deps, env, info, cluster_contract, asset, min_cluster),
        ExecuteMsg::Mint {
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => mint(
            deps,
            env,
            info,
            cluster_contract,
            &asset_amounts,
            min_tokens,
        ),
        ExecuteMsg::Redeem {
            cluster_contract,
            max_tokens,
            asset_amounts,
        } => redeem(deps, env, info, cluster_contract, max_tokens, asset_amounts),
    }
}

pub fn update_owner(deps: DepsMut, info: MessageInfo, owner: &String) -> StdResult<Response> {
    let cfg = read_config(deps.storage)?;

    if info.sender.to_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let mut new_cfg = cfg;
    new_cfg.owner = owner.clone();
    store_config(deps.storage, &new_cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_owner")]))
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let msg = cw20_msg.msg;
    let config: Config = read_config(deps.storage)?;

    match from_binary(&msg)? {
        Cw20HookMsg::DepositReward { rewards } => {
            // only reward token contract can execute this message
            if config.nebula_token != info.sender.to_string() {
                return Err(StdError::generic_err("unauthorized"));
            }

            let mut rewards_amount = Uint128::zero();
            for (_, _, amount) in rewards.iter() {
                rewards_amount += *amount;
            }

            if rewards_amount != cw20_msg.amount {
                return Err(StdError::generic_err("rewards amount miss matched"));
            }

            deposit_reward(deps, rewards, cw20_msg.amount)
        }
    }
}

pub fn new_penalty_period(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let cfg = read_config(deps.storage)?;

    if info.sender.to_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
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
        factory: state.factory,
        custody: state.custody,
        terraswap_factory: state.terraswap_factory,
        nebula_token: state.nebula_token,
        base_denom: state.base_denom,
        owner: state.owner,
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
    let pool_info = read_from_pool_bucket(&pool_bucket, &cluster_address);
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
    let contribution_bucket = contributions_read(deps.storage, &contributor_address, pool_type);
    let contributions = read_from_contribution_bucket(&contribution_bucket, &cluster_address);
    let resp = CurrentContributorInfoResponse {
        n: read_current_n(deps.storage)?,
        value_contributed: contributions.value_contributed,
    };
    Ok(resp)
}

pub fn query_contributor_pending_rewards(
    deps: Deps,
    contributor_address: String,
) -> StdResult<ContributorPendingRewardsResponse> {
    let pending_rewards = read_pending_rewards(deps.storage, &contributor_address);
    let resp = ContributorPendingRewardsResponse { pending_rewards };
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
