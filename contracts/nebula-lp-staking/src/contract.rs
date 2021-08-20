use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, InitResponse, MigrateResponse, MigrateResult, Querier, StdError, StdResult, Storage,
    Uint128,
};

use nebula_protocol::staking::{
    ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, MigrateMsg, PoolInfoResponse, QueryMsg,
};

use crate::rewards::{deposit_reward, query_reward_info, withdraw_reward};
use crate::staking::{auto_stake, auto_stake_hook, bond, unbond};
use crate::state::{read_config, read_pool_info, store_config, store_pool_info, Config, PoolInfo};

use cw20::Cw20ReceiveMsg;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: msg.owner,
            nebula_token: msg.nebula_token,
            terraswap_factory: msg.terraswap_factory,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::UpdateConfig { owner } => update_config(deps, env, owner),
        HandleMsg::RegisterAsset {
            asset_token,
            staking_token,
        } => register_asset(deps, env, asset_token, staking_token),
        HandleMsg::Unbond {
            asset_token,
            amount,
        } => unbond(deps, env.message.sender, asset_token, amount),
        HandleMsg::Withdraw { asset_token } => withdraw_reward(deps, env, asset_token),
        HandleMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, assets, slippage_tolerance),
        HandleMsg::AutoStakeHook {
            asset_token,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        } => auto_stake_hook(
            deps,
            env,
            asset_token,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        ),
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    if let Some(msg) = cw20_msg.msg {
        let config: Config = read_config(&deps.storage)?;

        match from_binary(&msg)? {
            Cw20HookMsg::Bond { asset_token } => {
                let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_token)?;

                // only staking token contract can execute this message
                if pool_info.staking_token != env.message.sender {
                    return Err(StdError::unauthorized());
                }

                bond(deps, env, cw20_msg.sender, asset_token, cw20_msg.amount)
            }
            Cw20HookMsg::DepositReward { rewards } => {
                // only reward token contract can execute this message
                if config.nebula_token != env.message.sender {
                    return Err(StdError::unauthorized());
                }

                let mut rewards_amount = Uint128::zero();
                for (_, amount) in rewards.iter() {
                    rewards_amount += *amount;
                }

                if rewards_amount != cw20_msg.amount {
                    return Err(StdError::generic_err("rewards amount miss matched"));
                }

                deposit_reward(deps, rewards, rewards_amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(&deps.storage)?;

    if env.message.sender != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

fn register_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_token: HumanAddr,
    staking_token: HumanAddr,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    if read_pool_info(&deps.storage, &asset_token).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        &mut deps.storage,
        &asset_token,
        &PoolInfo {
            staking_token,
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "register_asset"),
            log("asset_token", asset_token.as_str()),
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
        QueryMsg::PoolInfo { asset_token } => to_binary(&query_pool_info(deps, asset_token)?),
        QueryMsg::RewardInfo {
            staker_addr,
            asset_token,
        } => to_binary(&query_reward_info(deps, staker_addr, asset_token)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner,
        nebula_token: state.nebula_token,
    };

    Ok(resp)
}

pub fn query_pool_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset_token: HumanAddr,
) -> StdResult<PoolInfoResponse> {
    let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_token)?;
    Ok(PoolInfoResponse {
        asset_token,
        staking_token: pool_info.staking_token,
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> MigrateResult {
    Ok(MigrateResponse::default())
}
