use crate::state::read_total_staged_asset;
use basket_math::FPDecimal;
use cosmwasm_std::{Api, BalanceResponse, BankQuery, Decimal, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, Uint128, WasmQuery, to_binary};
use cw20::{BalanceResponse as Cw20BalanceResponse, TokenInfoResponse as Cw20TokenInfoResponse};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use terraswap::{asset::AssetInfo, querier::query_balance};
use log::info;

/// QueryMsgs to external contracts
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtQueryMsg {
    // Oracle
    Price {
        base_asset: AssetInfo,
        quote_asset: String,
    },
    // Cw20
    Balance {
        address: HumanAddr,
    },
    TokenInfo {},
}

#[derive(Serialize, Deserialize)]
pub struct PriceResponse {
    pub rate: Decimal,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

/// EXTERNAL QUERY
/// -- Queries the oracle contract for the current asset price
pub fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle_address: &HumanAddr,
    asset_info: &AssetInfo,
) -> StdResult<FPDecimal> {
    // perform query
    let res: PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: oracle_address.clone(),
        msg: to_binary(&ExtQueryMsg::Price {
            base_asset: asset_info.clone(),
            quote_asset: "uusd".to_string(),
        })?,
    }))?;

    // transform Decimal -> FPDecimal
    Ok(FPDecimal::from_str(res.rate.to_string().as_str())?)
}


// Query native balances
pub fn query_native_balance_minus_staged<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    denom: &String,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    let tot_balance = query_balance(&deps, account_address, denom.clone())?;

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
