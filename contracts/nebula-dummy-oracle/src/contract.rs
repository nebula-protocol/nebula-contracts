use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, InitResponse, Querier, StdResult,
    Storage,
};

use crate::msg::{HandleMsg, InitMsg, PriceResponse, QueryMsg};
use crate::state::{read_last_update_time, read_price, set_price, store_last_update_time};

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::SetPrices { prices } => try_set_prices(deps, env, &prices),
    }
}

pub fn try_set_prices<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    prices: &Vec<(String, Decimal)>,
) -> StdResult<HandleResponse> {
    for (asset, price) in prices.iter() {
        set_price(&mut deps.storage, asset, price)?;
    }
    store_last_update_time(&mut deps.storage, &env.block.time)?;
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { base_asset, .. } => to_binary(&query_price(deps, base_asset)?),
    }
}

fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset: String,
) -> StdResult<PriceResponse> {
    let rate = read_price(&deps.storage, &asset)?;
    let last_update = read_last_update_time(&deps.storage)?;

    Ok(PriceResponse {
        rate,
        last_updated_base: u64::MAX,
        last_updated_quote: last_update,
    })
}
