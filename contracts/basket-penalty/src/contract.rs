use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, InitResponse, Querier, StdResult,
    Storage, Uint128
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg, MintResponse, RedeemResponse};

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
    Ok(HandleResponse::default())
}

pub fn compute_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<Uint128>,
) -> StdResult<MintResponse> {
    Ok(
        MintResponse {
        mint_tokens: Uint128(1),
        log: vec![]
    })
}

pub fn compute_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    redeem_tokens: &Uint128,
    redeem_weights: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<Uint128>,
) -> StdResult<RedeemResponse> {

    Ok(
        RedeemResponse {
            redeem_assets: vec![Uint128(1); inventory.len()],
            log: vec![]
        })
}


pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Mint {
            basket_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights
        } => to_binary(&compute_mint(deps, &basket_token_supply, &inventory, &mint_asset_amounts, &asset_prices, &target_weights)?),
        QueryMsg::Redeem {
            basket_token_supply,
            inventory,
            redeem_tokens,
            redeem_weights,
            asset_prices,
            target_weights
        } => to_binary(&compute_redeem(deps, &basket_token_supply, &inventory, &redeem_tokens, &redeem_weights, &asset_prices, &target_weights)?),
    }
}
