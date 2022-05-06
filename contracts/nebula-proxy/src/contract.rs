#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::arbitrageurs::{arb_cluster_create, arb_cluster_redeem, send_all, swap_all};
use crate::error::ContractError;
use crate::rebalancers::{create, internal_rewarded_create, internal_rewarded_redeem, redeem};
use crate::state::{read_config, store_config, Config};
use cw2::set_contract_version;
use nebula_protocol::proxy::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-proxy";
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

    let mut config = Config {
        factory: deps.api.addr_validate(msg.factory.as_str())?,
        incentives: None,
        astroport_factory: deps.api.addr_validate(msg.astroport_factory.as_str())?,
        nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
        base_denom: msg.base_denom,
        owner: deps.api.addr_validate(msg.owner.as_str())?,
    };

    if let Some(incentives) = msg.incentives {
        config.incentives = Some(deps.api.addr_validate(incentives.as_str())?);
    }

    // Set the initial contract settings
    store_config(deps.storage, &config)?;
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
        ExecuteMsg::UpdateConfig { owner, incentives } => {
            update_config(deps, info, owner, incentives)
        }
        ExecuteMsg::_SwapAll {
            astroport_pair,
            cluster_token,
            to_ust,
            min_return,
            base_denom,
        } => swap_all(
            deps,
            env,
            info,
            astroport_pair,
            cluster_token,
            to_ust,
            min_return,
            base_denom,
        ),
        ExecuteMsg::_SendAll {
            asset_infos,
            send_to,
        } => send_all(deps, env, info, &asset_infos, send_to),
        ExecuteMsg::_InternalRewardedCreate {
            rebalancer,
            cluster_contract,
            incentives,
            asset_amounts,
            min_tokens,
        } => internal_rewarded_create(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            incentives,
            &asset_amounts,
            min_tokens,
        ),
        ExecuteMsg::_InternalRewardedRedeem {
            rebalancer,
            cluster_contract,
            cluster_token,
            incentives,
            max_tokens,
            asset_amounts,
        } => internal_rewarded_redeem(
            deps,
            env,
            info,
            rebalancer,
            cluster_contract,
            cluster_token,
            incentives,
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
/// Updates general contract parameters.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is an
///     address to claim the ownership of the contract.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    incentives: Option<Option<String>>,
) -> Result<Response, ContractError> {
    let mut config = read_config(deps.storage)?;

    // Permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // Validate address format
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    match incentives {
        Some(Some(incentives)) => {
            // Validate address format
            config.incentives = Some(deps.api.addr_validate(incentives.as_str())?)
        }
        Some(None) => config.incentives = None,
        None => (),
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
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
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let mut resp = ConfigResponse {
        factory: state.factory.to_string(),
        incentives: None,
        astroport_factory: state.astroport_factory.to_string(),
        nebula_token: state.nebula_token.to_string(),
        base_denom: state.base_denom,
        owner: state.owner.to_string(),
    };

    if let Some(incentives) = state.incentives {
        resp.incentives = Some(incentives.to_string())
    }

    Ok(resp)
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
