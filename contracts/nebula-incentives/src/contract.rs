#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};

use crate::error::ContractError;
use crate::handle_records::{record_astroport_impact, record_rebalancer_rewards};
use crate::handle_rewards::{deposit_reward, new_penalty_period, withdraw_reward};
use crate::query::{
    query_config, query_contributor_info, query_contributor_pending_rewards, query_penalty_period,
    query_pool_info,
};
use crate::state::{read_config, store_config, store_current_n, Config};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use nebula_protocol::incentives::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-incentives";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set the initial contract settings
    store_config(
        deps.storage,
        &Config {
            proxy: deps.api.addr_validate(msg.proxy.as_str())?,
            custody: deps.api.addr_validate(msg.custody.as_str())?,
            nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
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
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///         }** Updates general contract parameters.
///
/// - **ExecuteMsg::Receive (msg)** Receives CW20 tokens and executes a hook message.
///
/// - **ExecuteMsg::Withdraw {}** Withdraws rewards for the sender.
///
/// - **ExecuteMsg::NewPenaltyPeriod {} Increments the penalty period by one.
///
/// - **ExecuteMsg::RecordAstroportImpact {
///             arbitrageur,
///             astroport_pair,
///             cluster_contract,
///             pool_before,
///         }** Records arbitrage contribution for the reward distribution.
///
///  - **ExecuteMsg::RecordRebalancerRewards {
///             rebalancer,
///             cluster_contract,
///             original_imbalance,
///         }** Records rebalance contribution for the reward distribution.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, &owner),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::Withdraw {} => withdraw_reward(deps, info),
        ExecuteMsg::NewPenaltyPeriod {} => new_penalty_period(deps, info),
        ExecuteMsg::RecordAstroportImpact {
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
        ExecuteMsg::RecordRebalancerRewards {
            rebalancer,
            cluster_contract,
            original_inventory,
        } => record_rebalancer_rewards(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            original_inventory,
        ),
    }
}

/// ## Description
/// Updates general contract parameters.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is a reference to an object of type [`String`] which is an
///     address to claim the ownership of the contract.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: &str,
) -> Result<Response, ContractError> {
    // Validate the address
    let validated_owner = deps.api.addr_validate(owner)?;

    let cfg = read_config(deps.storage)?;

    // Permission check
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Change owner and save
    let mut new_cfg = cfg;
    new_cfg.owner = validated_owner;
    store_config(deps.storage, &new_cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
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
