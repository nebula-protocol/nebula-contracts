use crate::querier::load_token_balance;
use crate::state::{read_neb, read_owner, set_neb, set_owner};
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HumanAddr, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::incentives_custody::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_owner(deps.storage, &msg.owner)?;
    set_neb(deps.storage, &msg.neb_token)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::RequestNeb { amount } => request_neb(deps, env, amount),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, env, &owner),
    }
}

pub fn update_owner(deps: DepsMut, env: Env, owner: &HumanAddr) -> StdResult<Response> {
    let old_owner = read_owner(deps.storage)?;

    // check permission
    if env.message.sender != old_owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    set_owner(deps.storage, &owner)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("old_owner", old_owner),
        attr("new_owner", owner),
    ]))
}

pub fn request_neb(deps: DepsMut, env: Env, amount: Uint128) -> StdResult<Response> {
    if env.message.sender != read_owner(deps.storage)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: read_neb(deps.storage)?,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: env.message.sender.clone(),
                amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "request_neb"),
            attr("from", env.contract.address),
            attr("to", env.message.sender),
            attr("amount", amount),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { custody } => {
            let nebula_token = read_neb(deps.storage)?;
            let balance = load_token_balance(deps, &nebula_token, &custody)?;
            Ok(to_binary(&to_binary(&balance).unwrap())?)
        }
    }
}
