#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    Addr, attr, from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
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
            owner: deps.api.addr_validate(msg.owner.as_str())?,
            nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
            astroport_factory: deps.api.addr_validate(msg.astroport_factory.as_str())?,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, owner),
        ExecuteMsg::RegisterAsset {
            asset_token,
            staking_token,
        } => register_asset(deps, info, asset_token, staking_token),
        ExecuteMsg::Unbond {
            asset_token,
            amount,
        } => unbond(deps, info.sender.to_string(), asset_token, amount),
        ExecuteMsg::Withdraw { asset_token } => withdraw_reward(deps, info, asset_token),
        ExecuteMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, info, assets, slippage_tolerance),
        ExecuteMsg::AutoStakeHook {
            asset_token,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        } => auto_stake_hook(
            deps,
            env,
            info,
            asset_token,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        ),
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
            let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
            let pool_info: PoolInfo = read_pool_info(deps.storage, &validated_asset_token)?;

            // only staking token contract can execute this message
            if pool_info.staking_token != info.sender {
                return Err(StdError::generic_err("unauthorized"));
            }

            bond(deps, info, Addr::unchecked(cw20_msg.sender), validated_asset_token, cw20_msg.amount)
        }
        Cw20HookMsg::DepositReward { rewards } => {
            // only reward token contract can execute this message
            if config.nebula_token != info.sender.to_string() {
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

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    staking_token: String,
) -> StdResult<Response> {
    let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
    let validated_staking_token = deps.api.addr_validate(staking_token.as_str())?;

    let config: Config = read_config(deps.storage)?;

    if config.owner != info.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    if read_pool_info(deps.storage, &validated_asset_token).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        deps.storage,
        &validated_asset_token,
        &PoolInfo {
            staking_token: validated_staking_token,
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_asset"),
        attr("asset_token", validated_asset_token.to_string()),
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
        owner: state.owner.to_string(),
        nebula_token: state.nebula_token.to_string(),
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, asset_token: String) -> StdResult<PoolInfoResponse> {
    let pool_info: PoolInfo = read_pool_info(deps.storage, &deps.api.addr_validate(asset_token.as_str())?)?;
    Ok(PoolInfoResponse {
        asset_token,
        staking_token: pool_info.staking_token.to_string(),
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
