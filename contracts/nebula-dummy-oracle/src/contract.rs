use cosmwasm_std::{
    entry_point, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{read_last_update_time, read_price, set_price, store_last_update_time};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetPrices { prices } => try_set_prices(deps, env, &prices),
    }
}

pub fn try_set_prices(
    deps: DepsMut,
    env: Env,
    prices: &Vec<(String, Decimal)>,
) -> StdResult<Response> {
    for (asset, price) in prices.iter() {
        set_price(deps.storage, asset, price)?;
    }
    store_last_update_time(deps.storage, &(env.block.time.nanos() / 1_000_000_000))?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { base_asset, .. } => to_binary(&query_price(deps, base_asset)?),
    }
}

fn query_price(deps: Deps, asset: String) -> StdResult<PriceResponse> {
    let rate = read_price(deps.storage, &asset)?;
    let last_update = read_last_update_time(deps.storage)?;

    Ok(PriceResponse {
        rate,
        last_updated_base: u64::MAX,
        last_updated_quote: last_update,
    })
}
