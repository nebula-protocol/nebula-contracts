use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, MigrateResponse, MigrateResult, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::arbitrageurs::{
    arb_cluster_mint, arb_cluster_redeem, record_terraswap_impact, send_all, swap_all,
};
use crate::rebalancers::{
    internal_rewarded_mint, internal_rewarded_redeem, mint, record_rebalancer_rewards, redeem,
};
use crate::rewards::{deposit_reward, increment_n, withdraw_reward};
use crate::state::{Config, contributions_read, pool_info_read, read_config, read_current_n, read_from_contribution_bucket, read_from_pool_bucket, read_pending_rewards, store_config, store_current_n};
use cw20::Cw20ReceiveMsg;
use nebula_protocol::incentives::{ConfigResponse, ContributorPendingRewardsResponse, CurrentContributorInfoResponse, Cw20HookMsg, HandleMsg, IncentivesPoolInfoResponse, InitMsg, MigrateMsg, PenaltyPeriodResponse, PoolType, QueryMsg};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            factory: msg.factory,
            custody: msg.custody,
            terraswap_factory: msg.terraswap_factory,
            nebula_token: msg.nebula_token,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    store_current_n(&mut deps.storage, 0)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::UpdateOwner { owner } => update_owner(deps, env, &owner),
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::Withdraw {} => withdraw_reward(deps, env),
        HandleMsg::NewPenaltyPeriod {} => new_penalty_period(deps, env),
        HandleMsg::_SwapAll {
            terraswap_pair,
            cluster_token,
            to_ust,
        } => swap_all(deps, env, terraswap_pair, cluster_token, to_ust),
        HandleMsg::_SendAll {
            asset_infos,
            send_to,
        } => send_all(deps, env, &asset_infos, send_to),
        HandleMsg::_RecordTerraswapImpact {
            arbitrageur,
            terraswap_pair,
            cluster_contract,
            pool_before,
        } => record_terraswap_impact(
            deps,
            env,
            arbitrageur,
            terraswap_pair,
            cluster_contract,
            pool_before,
        ),
        HandleMsg::_RecordRebalancerRewards {
            rebalancer,
            cluster_contract,
            original_imbalance,
        } => record_rebalancer_rewards(deps, env, rebalancer, cluster_contract, original_imbalance),
        HandleMsg::_InternalRewardedMint {
            rebalancer,
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => internal_rewarded_mint(
            deps,
            env,
            rebalancer,
            cluster_contract,
            &asset_amounts,
            min_tokens,
        ),
        HandleMsg::_InternalRewardedRedeem {
            rebalancer,
            cluster_contract,
            cluster_token,
            max_tokens,
            asset_amounts,
        } => internal_rewarded_redeem(
            deps,
            env,
            rebalancer,
            cluster_contract,
            cluster_token,
            max_tokens,
            asset_amounts,
        ),
        HandleMsg::ArbClusterMint {
            cluster_contract,
            assets,
        } => arb_cluster_mint(deps, env, cluster_contract, &assets),
        HandleMsg::ArbClusterRedeem {
            cluster_contract,
            asset,
        } => arb_cluster_redeem(deps, env, cluster_contract, asset),
        HandleMsg::Mint {
            cluster_contract,
            asset_amounts,
            min_tokens,
        } => mint(deps, env, cluster_contract, &asset_amounts, min_tokens),
        HandleMsg::Redeem {
            cluster_contract,
            max_tokens,
            asset_amounts,
        } => redeem(deps, env, cluster_contract, max_tokens, asset_amounts),
    }
}

pub fn update_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let mut new_cfg = cfg;
    new_cfg.owner = owner.clone();
    store_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_owner")],
        data: None,
    })
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    if let Some(msg) = cw20_msg.msg {
        let config: Config = read_config(&deps.storage)?;

        match from_binary(&msg)? {
            Cw20HookMsg::DepositReward { rewards } => {
                // only reward token contract can execute this message
                if config.nebula_token != env.message.sender {
                    return Err(StdError::unauthorized());
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
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

pub fn new_penalty_period<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let n = read_current_n(&mut deps.storage)?;

    let new_n = increment_n(&mut deps.storage)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "new_penalty_period"),
            log("previous_n", n),
            log("current_n", new_n),
        ],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
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
        QueryMsg::ContributorPendingRewards { contributor_address } => to_binary(&query_contributor_pending_rewards(deps, contributor_address)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
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

pub fn query_penalty_period<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<PenaltyPeriodResponse> {
    let n = read_current_n(&deps.storage)?;
    let resp = PenaltyPeriodResponse { n };
    Ok(resp)
}

pub fn query_pool_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    pool_type: u16,
    cluster_address: HumanAddr,
    n: Option<u64>,
) -> StdResult<IncentivesPoolInfoResponse> {
    let n = match n {
        Some(_) => n.unwrap(),
        None => read_current_n(&deps.storage)?,
    };
    let pool_bucket = pool_info_read(&deps.storage, pool_type, n);
    let pool_info = read_from_pool_bucket(&pool_bucket, &cluster_address);
    let resp = IncentivesPoolInfoResponse {
        value_total: pool_info.value_total,
        reward_total: pool_info.reward_total,
    };
    Ok(resp)
}

pub fn query_contributor_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    pool_type: u16,
    contributor_address: HumanAddr,
    cluster_address: HumanAddr,
) -> StdResult<CurrentContributorInfoResponse> {
    let contribution_bucket = contributions_read(&deps.storage, &contributor_address, pool_type);
    let contributions = read_from_contribution_bucket(&contribution_bucket, &cluster_address);
    let resp = CurrentContributorInfoResponse {
        n: read_current_n(&deps.storage)?,
        value_contributed: contributions.value_contributed,
    };
    Ok(resp)
}

pub fn query_contributor_pending_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contributor_address: HumanAddr,
) -> StdResult<ContributorPendingRewardsResponse> {
    let pending_rewards = read_pending_rewards(&deps.storage, &contributor_address);
    let resp = ContributorPendingRewardsResponse {
        pending_rewards
    };
    Ok(resp)
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> MigrateResult {
    Ok(MigrateResponse::default())
}
