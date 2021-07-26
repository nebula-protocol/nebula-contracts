use crate::state::{read_neb, read_owner, set_neb, set_owner};
use cosmwasm_std::{
    log, to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
use nebula_protocol::incentives_custody::{HandleMsg, InitMsg, QueryMsg};

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
        HandleMsg::RequestNeb { amount } => request_neb(deps, env, amount),
        HandleMsg::UpdateOwner { owner } => update_owner(deps, env, &owner),
    }
}

pub fn update_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let old_owner = read_owner(&deps.storage)?;

    // check permission
    if env.message.sender != old_owner {
        return Err(StdError::unauthorized());
    }

    set_owner(&mut deps.storage, &owner)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_owner")],
        data: None,
    })
}

pub fn request_neb<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
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
