#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};

use nebula_protocol::staking::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PoolInfoResponse, QueryMsg,
};

use crate::error::ContractError;
use crate::rewards::{deposit_reward, query_reward_info, withdraw_reward};
use crate::staking::{auto_stake, auto_stake_hook, bond, unbond};
use crate::state::{read_config, read_pool_info, store_config, store_pool_info, Config, PoolInfo};

use cw20::Cw20ReceiveMsg;

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
/// - **ExecuteMsg::Receive (msg)** Receives CW20 tokens and executes a hook message.
///
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///         }** Updates general LP staking contract parameters.
///
/// - **ExecuteMsg::RegisterAsset {
///             asset_token,
///             staking_token,
///         }** Registers a new LP staking token contract.
///
/// - **ExecuteMsg::Unbond {
///             asset_token,
///             amount,
///         }** Unbond staked LP tokens for the specified amount.
///
/// - **ExecuteMsg::Withdraw {
///             asset_token,
///         }** Withdraws all rewards or single reward depending on asset_token.
///
/// - **ExecuteMsg::AutoStake {
///             assets,
///             slippage_tolerance,
///         }** Provides liquidity and automatically stakes the LP tokens.
///
/// - **ExecuteMsg::AutoStakeHook {
///             asset_token,
///             staking_token,
///             staker_addr,
///             prev_staking_token_amount,
///         }** Stakes the minted LP tokens.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
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
        // `Bond` stakes the sent LP token
        Cw20HookMsg::Bond { asset_token } => {
            // Validate address format
            let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
            // Get the LP staking pool info
            let pool_info: PoolInfo = read_pool_info(deps.storage, &validated_asset_token)?;

            // Permission check - only staking token contract can execute this message
            if pool_info.staking_token != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            // Bond the sent LP tokens
            bond(
                deps,
                info,
                Addr::unchecked(cw20_msg.sender),
                validated_asset_token,
                cw20_msg.amount,
            )
        }
        // `DepositReward` adds reward to LP staking pools
        Cw20HookMsg::DepositReward { rewards } => {
            // Permission check - only Nebula token contract can execute this message
            if config.nebula_token != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            // Check the reward amount
            let mut rewards_amount = Uint128::zero();
            for (_, amount) in rewards.iter() {
                rewards_amount += *amount;
            }

            if rewards_amount != cw20_msg.amount {
                return Err(ContractError::Generic(
                    "Rewards amount miss matched".to_string(),
                ));
            }
            // Desposit the reward
            deposit_reward(deps, rewards, rewards_amount)
        }
    }
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is the contract owner.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // Validate address format
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Registers a new LP staking token contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_token** is an object of type [`String`] which is an address
///     of a cluster token contract.
///
/// - **staking_token** is an object of type [`String`] which is an address
///     of a cluster LP token contract.
///
/// ## Executor
/// Only the owner can execute this.
fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    staking_token: String,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
    let validated_staking_token = deps.api.addr_validate(staking_token.as_str())?;

    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Check if the pair (cluster token, cluster LP token) is not registered
    if read_pool_info(deps.storage, &validated_asset_token).is_ok() {
        return Err(ContractError::Generic(
            "Asset was already registered".to_string(),
        ));
    }

    // Register the pair (cluster token, cluster LP token)
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
/// - **QueryMsg::PoolInfo { asset_token }** Returns information of a LP staking pool.
///
/// - **QueryMsg::RewardInfo {
///             staker_addr,
///             asset_token,
///         }** Returns reward information of a LP staker from a specific LP staking pool.
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

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        astroport_factory: state.astroport_factory.to_string(),
        nebula_token: state.nebula_token.to_string(),
    };

    Ok(resp)
}

/// ## Description
/// Returns information of a LP staking pool.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **asset_token** is an object of type [`String`] which is an address of
///     a cluster token contract.
pub fn query_pool_info(deps: Deps, asset_token: String) -> StdResult<PoolInfoResponse> {
    // Read a LP staking pool information corresponding to the provided cluster token
    let pool_info: PoolInfo =
        read_pool_info(deps.storage, &deps.api.addr_validate(asset_token.as_str())?)?;
    Ok(PoolInfoResponse {
        asset_token,
        staking_token: pool_info.staking_token.to_string(),
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
    })
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
