#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::querier::load_token_balance;
use crate::state::{read_neb, read_owner, set_neb, set_owner};
use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;
use nebula_protocol::incentives_custody::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-incentives-custody";
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

    // Set contract owner
    set_owner(deps.storage, &deps.api.addr_validate(msg.owner.as_str())?)?;
    // Register Nebula token contract
    set_neb(
        deps.storage,
        &deps.api.addr_validate(msg.nebula_token.as_str())?,
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
/// - **ExecuteMsg::UpdateConfig {
///             owner
///         }** Updates general contract parameters.
///
/// - **ExecuteMsg::RequestNeb {
///             amount
///         }** Sends Nebula tokens to the message sender.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, &owner),
        ExecuteMsg::RequestNeb { amount } => request_neb(deps, env, info, amount),
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
/// - **owner** is a reference to an object of type [`str`] which is the owner to update.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: &str,
) -> Result<Response, ContractError> {
    let old_owner = read_owner(deps.storage)?;

    // Permission check
    if info.sender != old_owner {
        return Err(ContractError::Unauthorized {});
    }

    // Set new owner
    set_owner(deps.storage, &deps.api.addr_validate(owner)?)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_config"),
        attr("old_owner", old_owner.to_string()),
        attr("new_owner", owner.to_string()),
    ]))
}

/// ## Description
/// Sends Nebula tokens to the message sender.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **amount** is an object of type [`Uint128`] which is an amount of Nebula token requested.
///
/// ## Executor
/// Only the owner can execute this.
pub fn request_neb(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != read_owner(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    // Send Nebula tokens to the message sender
    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: read_neb(deps.storage)?.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "request_neb"),
            attr("from", env.contract.address.to_string()),
            attr("to", info.sender.to_string()),
            attr("amount", amount),
        ]))
}

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Balance {}** Returns the current Nebula token balance of the incentives custody contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance {} => {
            // Get Nebula token contract
            let nebula_token = read_neb(deps.storage)?;
            // Query Nebula balance of the contract
            let balance = load_token_balance(deps, &nebula_token, &env.contract.address)?;
            Ok(to_binary(&balance)?)
        }
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner = read_owner(deps.storage)?;
    let resp = ConfigResponse {
        owner: owner.to_string(),
    };

    Ok(resp)
}
