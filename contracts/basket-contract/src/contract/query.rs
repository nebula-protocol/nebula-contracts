use cosmwasm_std::{to_binary, Api, Binary, Extern, Querier, StdResult, Storage};

use crate::msg::{QueryMsg, TargetResponse};
use crate::state::read_target;

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTarget {} => to_binary(&query_target(deps)?),
    }
}

fn query_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TargetResponse> {
    let target = read_target(&deps.storage)?;
    Ok(TargetResponse { target })
}
