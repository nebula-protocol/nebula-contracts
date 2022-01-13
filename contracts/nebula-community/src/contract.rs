#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::state::{read_config, store_config, Config};
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use nebula_protocol::community::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use astroport::asset::Asset;

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
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => {
            update_config(deps, info, owner)
        }
        ExecuteMsg::Spend { asset, recipient } => {
            spend(deps, info, asset, recipient)
        },
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != info.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    config.owner = deps.api.addr_validate(owner.as_str())?;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// Spend
/// Owner can execute spend operation to send
/// `amount` of NEB token to `recipient` for community purpose
pub fn spend(
    deps: DepsMut,
    info: MessageInfo,
    asset: Asset,
    recipient: String,
) -> StdResult<Response> {
    let validated_recipient = deps.api.addr_validate(recipient.as_str())?;

    let config: Config = read_config(deps.storage)?;
    if config.owner != info.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(Response::new()
        .add_messages(vec![asset.clone().into_msg(&deps.querier, validated_recipient)?])
        .add_attributes(vec![
            attr("action", "spend"),
            attr("asset", asset.to_string()),
            attr("recipient", recipient),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
