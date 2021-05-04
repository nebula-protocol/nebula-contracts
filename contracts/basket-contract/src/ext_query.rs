use crate::state::read_total_staged_asset;
use cosmwasm_std::{
    to_binary, Api, Decimal, Extern, HumanAddr, LogAttribute, Querier, QueryRequest, StdResult,
    Storage, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, TokenInfoResponse as Cw20TokenInfoResponse};
use serde::{Deserialize, Serialize};
use terraswap::{asset::AssetInfo, querier::query_balance};

/// QueryMsgs to external contracts
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtQueryMsg {
    // Oracle
    Price {
        base_asset: String,
        quote_asset: String,
    },
    // Cw20
    Balance {
        address: HumanAddr,
    },
    TokenInfo {},

    // Penalty mint
    Mint {
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        mint_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },

    // Penalty redeem
    Redeem {
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        max_tokens: Uint128,
        redeem_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct PriceResponse {
    pub rate: Decimal,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

#[derive(Serialize, Deserialize)]
pub struct MintResponse {
    pub mint_tokens: Uint128,
    pub log: Vec<LogAttribute>,
}

#[derive(Serialize, Deserialize)]
pub struct RedeemResponse {
    pub redeem_assets: Vec<Uint128>,
    pub token_cost: Uint128,
    pub log: Vec<LogAttribute>,
}

/// EXTERNAL QUERY
/// -- Queries the oracle contract for the current asset price
pub fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle_address: &HumanAddr,
    asset_info: &AssetInfo,
) -> StdResult<String> {
    // perform query
    let res: PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: oracle_address.clone(),
        msg: to_binary(&ExtQueryMsg::Price {
            base_asset: asset_info.to_string(),
            quote_asset: "uusd".to_string(),
        })?,
    }))?;

    Ok(res.rate.to_string().as_str().parse().unwrap())
}

// Query native balances
pub fn query_native_balance_minus_staged<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    denom: &String,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    let tot_balance = query_balance(&deps, account_address, denom.clone())?;
    // let all_balances = query_all_balances(&deps, account_address)?;
    // println!("me balances {:?}", all_balances);
    let staged_balance = read_total_staged_asset(
        &deps.storage,
        &AssetInfo::NativeToken {
            denom: denom.clone(),
        },
    )?;

    tot_balance - staged_balance
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current balance of an account without
/// the counting the staged amount
pub fn query_cw20_balance_minus_staged<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset_address: &HumanAddr,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    let tot_balance = query_cw20_balance(&deps, &asset_address, &account_address)?;

    let staged_balance = read_total_staged_asset(
        &deps.storage,
        &AssetInfo::Token {
            contract_addr: asset_address.clone(),
        },
    )?;

    tot_balance - staged_balance
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current balance of account
pub fn query_cw20_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset_address: &HumanAddr,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.clone(),
        msg: to_binary(&ExtQueryMsg::Balance {
            address: account_address.clone(),
        })?,
    }))?;

    Ok(res.balance)
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current total supply
pub fn query_cw20_token_supply<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset_address: &HumanAddr,
) -> StdResult<Uint128> {
    let res: Cw20TokenInfoResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.clone(),
        msg: to_binary(&ExtQueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// EXTERNAL QUERY
/// -- Queries the penalty contract for the amount to mint
pub fn query_mint_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    penalty_address: &HumanAddr,
    basket_token_supply: Uint128,
    inventory: Vec<Uint128>,
    mint_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<u32>,
) -> StdResult<MintResponse> {
    let res: MintResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.clone(),
        msg: to_binary(&ExtQueryMsg::Mint {
            basket_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))?;

    Ok(res)
}

/// EXTERNAL QUERY
/// -- Queries the penalty contract for the amount to mint
pub fn query_redeem_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    penalty_address: &HumanAddr,
    basket_token_supply: Uint128,
    inventory: Vec<Uint128>,
    max_tokens: Uint128,
    redeem_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<u32>,
) -> StdResult<RedeemResponse> {
    let res: RedeemResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.clone(),
        msg: to_binary(&ExtQueryMsg::Redeem {
            basket_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))?;

    Ok(res)
}
