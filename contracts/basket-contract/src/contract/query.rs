use cosmwasm_std::{to_binary, Api, Binary, Extern, HumanAddr, Querier, StdResult, Storage};

use crate::msg::{ConfigResponse, QueryMsg, StagedAmountResponse, TargetResponse};
use crate::state::{read_config, read_staged_asset, read_target};

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Target {} => to_binary(&query_target(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::StagedAmount { account, asset } => {
            to_binary(&query_staged_amount(deps, &account, &asset)?)
        }
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let cfg = read_config(&deps.storage)?;
    Ok(ConfigResponse { config: cfg })
}

fn query_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TargetResponse> {
    let target = read_target(&deps.storage)?;
    Ok(TargetResponse { target })
}

fn query_staged_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    asset: &HumanAddr,
) -> StdResult<StagedAmountResponse> {
    let staged_amount = read_staged_asset(&deps.storage, account, asset)?;
    Ok(StagedAmountResponse { staged_amount })
}

pub fn query_basket_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>
) -> StdResult<BasketStateResponse> {
    // penalty parameters
    // total outstanding basket tokens
    // oracle prices for all assets (uusd)
    // total inventory of all
    // current target weights
    // block height
    Ok(BasketState {
        asset_len: ,
        asset_token: ,
        inventory: ,
        oracle_price: ,
        target_weight: ,
    })
}