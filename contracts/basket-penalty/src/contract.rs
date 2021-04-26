use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, InitResponse, Querier, StdResult,
    Storage, Uint128
};

use basket_math::{dot, sum, FPDecimal};
use crate::msg::{HandleMsg, InitMsg, PriceResponse, QueryMsg, MintResponse};
use crate::state::{read_price, set_price};

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
    _env: Env,
    prices: &Vec<(String, Decimal)>,
) -> StdResult<HandleResponse> {
    for (asset, price) in prices.iter() {
        set_price(&mut deps.storage, asset, price)?;
    }
    Ok(HandleResponse::default())
}


pub fn compute_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    inventory: &Vec<Uint128>,
    mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<Uint128>,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse::default())
}

pub fn compute_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    inventory: &Vec<Uint128>,
    redeem_tokens: &Uint128,
    redeem_weights: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<Uint128>,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse::default())
}


pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<HandleResponse> {
    match msg {
        QueryMsg::Mint {
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights
        } => compute_mint(deps, &inventory, &mint_asset_amounts, &asset_prices, &target_weights),
        QueryMsg::Redeem {
            inventory,
            redeem_tokens,
            redeem_weights,
            asset_prices,
            target_weights
        } => compute_redeem(deps, &inventory, &redeem_tokens, &redeem_weights, &asset_prices, &target_weights),
    }
}

fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset: String,
) -> StdResult<PriceResponse> {
    let rate = read_price(&deps.storage, &asset)?;
    Ok(PriceResponse {
        rate,
        last_updated_base: 0,
        last_updated_quote: 0,
    })
}
