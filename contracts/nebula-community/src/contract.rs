#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::state::{read_config, store_config, Config};
use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};

use astroport::asset::Asset;
use nebula_protocol::community::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

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
/// - **_env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             token_code_id,
///             cluster_code_id,
///             distribution_schedule,
///         }** Updates general community contract parameters.
///
/// - **ExecuteMsg::Spend {
///             asset,
///             recipient,
///         }** Sends `asset` to `recipient` for community purpose.
///
/// - **ExecuteMsg::PassCommand {
///             contract_addr,
///             msg,
///         }** Passes a command for the community contract to execute.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, owner),
        ExecuteMsg::Spend { asset, recipient } => spend(deps, info, asset, recipient),
        ExecuteMsg::PassCommand { contract_addr, msg } => {
            pass_command(deps, info, contract_addr, msg)
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
/// - **owner** is an object of type [`Option<String>`] which is an owner to update.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Validate address format
    config.owner = deps.api.addr_validate(owner.as_str())?;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Executes spend operation to send `asset` to `recipient` for community purpose.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset** is an object of type [`Asset`] which is an asset to send.
///
/// - **recipient** is an object of type [`String`] which is an address of the recipient.
///
/// ## Executor
/// Only the owner can execute this.
pub fn spend(
    deps: DepsMut,
    info: MessageInfo,
    asset: Asset,
    recipient: String,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_recipient = deps.api.addr_validate(recipient.as_str())?;

    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new()
        .add_messages(vec![asset
            .clone()
            .into_msg(&deps.querier, validated_recipient)?])
        .add_attributes(vec![
            attr("action", "spend"),
            attr("asset", asset.to_string()),
            attr("recipient", recipient),
        ]))
}

/// ## Description
/// Passes a command for the community contract to execute.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **contract_addr** is an object of type [`String`] which is an address of the
///     target contract to execute the command on.
///
/// - **msg** is an object of type [`Binary`] which is an encoded message / command
///     to be executed on the target contract.
///
/// ## Executor
/// Only the owner can execute this.
pub fn pass_command(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(
        // Execute `msg` on `contract_addr` with the community contract as the sender
        Response::new().add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds: vec![],
        })]),
    )
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
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
    };

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
