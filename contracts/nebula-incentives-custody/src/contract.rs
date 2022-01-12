#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::querier::load_token_balance;
use crate::state::{read_neb, read_owner, set_neb, set_owner};
use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::incentives_custody::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_owner(deps.storage, &deps.api.addr_validate(msg.owner.as_str())?)?;
    set_neb(
        deps.storage,
        &deps.api.addr_validate(msg.nebula_token.as_str())?,
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::RequestNeb { amount } => request_neb(deps, env, info, amount),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, &owner),
    }
}

pub fn update_owner(deps: DepsMut, info: MessageInfo, owner: &String) -> StdResult<Response> {
    let old_owner = read_owner(deps.storage)?;

    // check permission
    if info.sender != old_owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    set_owner(deps.storage, &deps.api.addr_validate(owner.as_str())?)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("old_owner", old_owner.to_string()),
        attr("new_owner", owner.to_string()),
    ]))
}

pub fn request_neb(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> StdResult<Response> {
    if info.sender != read_owner(deps.storage)? {
        return Err(StdError::generic_err("unauthorized"));
    }

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { custody } => {
            let nebula_token = read_neb(deps.storage)?;
            let balance = load_token_balance(
                deps,
                &nebula_token,
                &deps.api.addr_validate(custody.as_str())?,
            )?;
            Ok(to_binary(&to_binary(&balance).unwrap())?)
        }
    }
}
