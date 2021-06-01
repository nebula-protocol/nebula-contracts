use cosmwasm_std::{to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, StdError, CosmosMsg, WasmMsg, Uint128, HumanAddr, log};
use cw20::Cw20HandleMsg;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{set_owner, read_owner, set_neb, read_neb};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_owner(&mut deps.storage, &msg.owner)?;
    set_neb(&mut deps.storage, &msg.neb_token)?;
    Ok(InitResponse::default())
}


pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::RequestNeb { amount } => handle_request_neb(deps, env, amount),
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, env, &owner),
    }
}

pub fn try_reset_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {

    let old_owner = read_owner(&deps.storage)?;

    // check permission
    if env.message.sender != old_owner {
        return Err(StdError::unauthorized());
    }

    set_owner(&mut deps.storage, &owner);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_try_reset_owner"),
        ],
        data: None,
    })
}

pub fn handle_request_neb<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128
) -> StdResult<HandleResponse> {

    if env.message.sender != read_owner(&deps.storage)? {
        return Err(StdError::unauthorized());
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: read_neb(&deps.storage)?,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: env.message.sender,
                amount,
            })?,
            send: vec![],
        })],
        log: vec![],
        data: None,
    })
}


pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _msg: QueryMsg,
) -> StdResult<Binary> {
    Ok(Binary::from(vec![0u8]))
}