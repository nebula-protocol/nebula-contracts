#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
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
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(&msg.owner)?,
            nebula_token: deps.api.addr_canonicalize(&msg.nebula_token)?,
            terraswap_factory: deps.api.addr_canonicalize(&msg.terraswap_factory)?,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateConfig { owner } => {
            let owner_addr = if let Some(owner_addr) = owner {
                Some(deps.api.addr_validate(&owner_addr)?)
            } else {
                None
            };
            update_config(deps, info, owner_addr)
        }
        ExecuteMsg::RegisterAsset {
            asset_token,
            staking_token,
        } => {
            let api = deps.api;
            register_asset(
                deps,
                info,
                api.addr_validate(&asset_token)?,
                api.addr_validate(&staking_token)?,
            )
        }
        ExecuteMsg::Unbond {
            asset_token,
            amount,
        } => unbond(deps, info.sender.to_string(), asset_token, amount),
        ExecuteMsg::Withdraw { asset_token } => {
            let asset_addr = if let Some(asset_addr) = asset_token {
                Some(deps.api.addr_validate(&asset_addr)?)
            } else {
                None
            };
            withdraw_reward(deps, info, asset_addr)
        }
        ExecuteMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, info, assets, slippage_tolerance),
        ExecuteMsg::AutoStakeHook {
            asset_token,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        } => {
            let api = deps.api;
            auto_stake_hook(
                deps,
                env,
                info,
                api.addr_validate(&asset_token)?,
                api.addr_validate(&staking_token)?,
                api.addr_validate(&staker_addr)?,
                prev_staking_token_amount,
            )
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let msg = cw20_msg.msg;
    let config: Config = read_config(deps.storage)?;

    match from_binary(&msg)? {
        Cw20HookMsg::Bond { asset_token } => {
            let pool_info: PoolInfo =
                read_pool_info(deps.storage, &deps.api.addr_canonicalize(&asset_token)?)?;

            // only staking token contract can execute this message
            if pool_info.staking_token != deps.api.addr_canonicalize(info.sender.as_str())? {
                return Err(StdError::generic_err("unauthorized"));
            }
            let api = deps.api;
            bond(
                deps,
                info,
                api.addr_validate(cw20_msg.sender.as_str())?,
                api.addr_validate(asset_token.as_str())?,
                cw20_msg.amount,
            )
        }
        Cw20HookMsg::DepositReward { rewards } => {
            // only reward token contract can execute this message
            if config.nebula_token != deps.api.addr_canonicalize(info.sender.as_str())? {
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

pub fn update_config(deps: DepsMut, info: MessageInfo, owner: Option<Addr>) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(owner.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: Addr,
    staking_token: Addr,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let asset_token_raw = deps.api.addr_canonicalize(asset_token.as_str())?;

    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    if read_pool_info(deps.storage, &asset_token_raw).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        deps.storage,
        &asset_token_raw,
        &PoolInfo {
            staking_token: deps.api.addr_canonicalize(staking_token.as_str())?,
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
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        nebula_token: deps.api.addr_humanize(&state.nebula_token)?.to_string(),
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, asset_token: String) -> StdResult<PoolInfoResponse> {
    let asset_token_raw = deps.api.addr_canonicalize(&asset_token)?;
    let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_token_raw)?;
    Ok(PoolInfoResponse {
        asset_token,
        staking_token: deps
            .api
            .addr_humanize(&pool_info.staking_token)?
            .to_string(),
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
