use cosmwasm_std::{
    to_binary, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmQuery,
};
use cw20::Cw20QueryMsg;
use cw20::{BalanceResponse as Cw20BalanceResponse, TokenInfoResponse as Cw20TokenInfoResponse};
use nebula_protocol::{
    cluster_factory::ConfigResponse as FactoryConfigResponse,
    cluster_factory::QueryMsg as FactoryQueryMsg, oracle::PriceResponse,
    oracle::QueryMsg as OracleQueryMsg, penalty::MintResponse,
    penalty::QueryMsg as PenaltyQueryMsg, penalty::RedeemResponse,
};
use terraswap::asset::AssetInfo;

/// EXTERNAL QUERY
/// -- Queries the oracle contract for the current asset price
pub fn query_price(
    querier: &QuerierWrapper,
    pricing_oracle_address: &String,
    asset_info: &AssetInfo,
    // prices from before < stale_threshold are considered stale
    // and result in an error
    _stale_threshold: u64,
) -> StdResult<String> {
    // perform query
    let res: PriceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pricing_oracle_address.clone().to_string(),
        msg: to_binary(&OracleQueryMsg::Price {
            base_asset: asset_info.to_string(),
            quote_asset: "uusd".to_string(),
        })?,
    }))?;

    // TODO: UNCOMMENT THIS WHEN DEPLOYING
    // if min(res.last_updated_quote, res.last_updated_base) < stale_threshold {
    //     return Err(StdError::generic_err(format!("oracle prices are stale")));
    // }
    Ok(res.rate.to_string().as_str().parse().unwrap())
}

/// EXTERNAL QUERY
/// -- Queries the asset balance of account
pub fn query_asset_balance(
    querier: &QuerierWrapper,
    account_address: &String,
    asset_info: &AssetInfo,
) -> StdResult<Uint128> {
    match asset_info {
        AssetInfo::Token { contract_addr } => {
            query_cw20_balance(querier, &(contract_addr), &account_address)
        }
        AssetInfo::NativeToken { denom } => query_balance(querier, &account_address, denom.clone()),
    }
}

/// EXTERNAL QUERY
/// -- Queries the native token balance of account
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: &String,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current balance of account
pub fn query_cw20_balance(
    querier: &QuerierWrapper,
    asset_address: &String,
    account_address: &String,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.clone().to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_address.clone().to_string(),
        })?,
    }))?;

    Ok(res.balance)
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current total supply
pub fn query_cw20_token_supply(
    querier: &QuerierWrapper,
    asset_address: &String,
) -> StdResult<Uint128> {
    let res: Cw20TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.clone().to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// EXTERNAL QUERY
/// -- Queries the cluster factory contract for the current total supply
pub fn query_collector_contract_address(
    querier: &QuerierWrapper,
    factory_address: &String,
) -> StdResult<(String, String)> {
    let res: FactoryConfigResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_address.clone().to_string(),
        msg: to_binary(&FactoryQueryMsg::Config {})?,
    }))?;

    Ok((res.commission_collector, res.protocol_fee_rate))
}

/// EXTERNAL QUERY
/// -- Queries the penalty contract for the amount to mint
pub fn query_mint_amount(
    querier: &QuerierWrapper,
    penalty_address: &String,
    block_height: u64,
    cluster_token_supply: Uint128,
    inventory: Vec<Uint128>,
    mint_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<Uint128>,
) -> StdResult<MintResponse> {
    let res: MintResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.clone().to_string(),
        msg: to_binary(&PenaltyQueryMsg::PenaltyQueryCreate {
            block_height,
            cluster_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))?;

    Ok(res)
}

/// EXTERNAL QUERY
/// -- Queries the penalty contract for the amount to redeem
pub fn query_redeem_amount(
    querier: &QuerierWrapper,
    penalty_address: &String,
    block_height: u64,
    cluster_token_supply: Uint128,
    inventory: Vec<Uint128>,
    max_tokens: Uint128,
    redeem_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<Uint128>,
) -> StdResult<RedeemResponse> {
    let res: RedeemResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.clone().to_string(),
        msg: to_binary(&PenaltyQueryMsg::PenaltyQueryRedeem {
            block_height,
            cluster_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))?;

    Ok(res)
}
