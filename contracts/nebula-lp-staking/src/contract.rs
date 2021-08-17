use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, HumanAddr,
    MessageInfo, Response, StdError, StdResult, Uint128,
};

use nebula_protocol::staking::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PoolInfoResponse, QueryMsg,
};

use crate::rewards::{deposit_reward, query_reward_info, withdraw_reward};
use crate::staking::{auto_stake, auto_stake_hook, bond, unbond};
use crate::state::{read_config, read_pool_info, store_config, store_pool_info, Config, PoolInfo};

use cw20::Cw20ReceiveMsg;

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
            owner: msg.owner,
            nebula_token: msg.nebula_token,
            terraswap_factory: msg.terraswap_factory,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, msg),
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, env, owner),
        ExecuteMsg::RegisterAsset {
            asset_token,
            staking_token,
        } => register_asset(deps, env, asset_token, staking_token),
        ExecuteMsg::Unbond {
            asset_token,
            amount,
        } => unbond(deps, env.message.sender, asset_token, amount),
        ExecuteMsg::Withdraw { asset_token } => withdraw_reward(deps, env, asset_token),
        ExecuteMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, assets, slippage_tolerance),
        ExecuteMsg::AutoStakeHook {
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

pub fn receive_cw20(deps: DepsMut, env: Env, cw20_msg: Cw20ReceiveMsg) -> StdResult<Response> {
    let msg = cw20_msg.msg;
    let config: Config = read_config(deps.storage)?;

    match from_binary(&msg)? {
        Cw20HookMsg::Bond { asset_token } => {
            let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_token)?;

            // only staking token contract can execute this message
            if pool_info.staking_token != env.message.sender {
                return Err(StdError::generic_err("unauthorized"));
            }

            bond(deps, env, cw20_msg.sender, asset_token, cw20_msg.amount)
        }
        Cw20HookMsg::DepositReward { rewards } => {
            // only reward token contract can execute this message
            if config.nebula_token != env.message.sender {
                return Err(StdError::generic_err("unauthorized"));
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
}

pub fn update_config(deps: DepsMut, env: Env, owner: Option<HumanAddr>) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if env.message.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

fn register_asset(
    deps: DepsMut,
    env: Env,
    asset_token: HumanAddr,
    staking_token: HumanAddr,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != env.message.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    if read_pool_info(deps.storage, &asset_token).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        deps.storage,
        &asset_token,
        &PoolInfo {
            staking_token,
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_asset"),
        attr("asset_token", asset_token.as_str()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PoolInfo { asset_token } => to_binary(&query_pool_info(deps, asset_token)?),
        QueryMsg::RewardInfo {
            staker_addr,
            asset_token,
        } => to_binary(&query_reward_info(deps, staker_addr, asset_token)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner,
        nebula_token: state.nebula_token,
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, asset_token: HumanAddr) -> StdResult<PoolInfoResponse> {
    let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_token)?;
    Ok(PoolInfoResponse {
        asset_token,
        staking_token: pool_info.staking_token,
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
