use astroport::asset::AssetInfo;
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankQuery, QuerierWrapper, QueryRequest, StdError, StdResult,
    Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use cw20::{BalanceResponse as Cw20BalanceResponse, TokenInfoResponse as Cw20TokenInfoResponse};
use nebula_protocol::{
    cluster_factory::ConfigResponse as FactoryConfigResponse,
    cluster_factory::QueryMsg as FactoryQueryMsg, oracle::PriceResponse,
    oracle::QueryMsg as OracleQueryMsg, penalty::PenaltyCreateResponse,
    penalty::PenaltyRedeemResponse, penalty::QueryMsg as PenaltyQueryMsg,
};

//////////////////////////////////////////////////////////////////////
/// EXTERNAL QUERY (to other contracts)
//////////////////////////////////////////////////////////////////////

/// ## Description
/// Queries the oracle contract for the current asset price.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **pricing_oracle_address** is a reference to an object of type [`Addr`].
///
/// - **asset_info** is a reference to an object of type [`AssetInfo`].
///
/// - **stale_threshold** is an object of type [`u64`].
///
/// - **quote_denom** is an object of type [`String`].
pub fn query_price(
    querier: &QuerierWrapper,
    pricing_oracle_address: &Addr,
    asset_info: &AssetInfo,
    // Prices from before < stale_threshold are considered stale
    // and result in an error
    stale_threshold: u64,
    quote_denom: &String,
) -> StdResult<String> {
    // Perform query
    let res: PriceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pricing_oracle_address.to_string(),
        msg: to_binary(&OracleQueryMsg::Price {
            base_asset: asset_info.clone(),
            quote_asset: AssetInfo::NativeToken {
                denom: quote_denom.to_string(),
            },
        })?,
    }))?;

    if std::cmp::min(res.last_updated_quote, res.last_updated_base) < stale_threshold {
        return Err(StdError::generic_err("oracle prices are stale".to_string()));
    }
    Ok(res.rate.to_string().as_str().parse().unwrap())
}

/// ## Description
/// Queries a specific asset balance of the given account.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **account_address** is a reference to an object of type [`Addr`].
///
/// - **asset_info** is a reference to an object of type [`AssetInfo`].
pub fn query_asset_balance(
    querier: &QuerierWrapper,
    account_address: &Addr,
    asset_info: &AssetInfo,
) -> StdResult<Uint128> {
    match asset_info {
        AssetInfo::Token { contract_addr } => {
            query_cw20_balance(querier, account_address, contract_addr)
        }
        AssetInfo::NativeToken { denom } => query_balance(querier, account_address, denom.clone()),
    }
}

/// ## Description
/// Queries the native token balance of the given account.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **account_address** is a reference to an object of type [`Addr`].
///
/// - **denom** is an object of type [`String`].
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: &Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

/// ## Description
/// Queries the token_address contract for the current balance of the given account.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **account_address** is a reference to an object of type [`Addr`].
///
/// - **asset_address** is a reference to an object of type [`Addr`].
pub fn query_cw20_balance(
    querier: &QuerierWrapper,
    account_address: &Addr,
    asset_address: &Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_address.to_string(),
        })?,
    }))?;

    Ok(res.balance)
}

/// ## Description
/// Queries the token_address contract for the token's current total supply.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **asset_address** is a reference to an object of type [`Addr`].
pub fn query_cw20_token_supply(
    querier: &QuerierWrapper,
    asset_address: &Addr,
) -> StdResult<Uint128> {
    let res: Cw20TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: asset_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res.total_supply)
}

/// ## Description
/// Queries the cluster factory config
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **factory_address** is a reference to an object of type [`Addr`].
pub fn query_factory_config(
    querier: &QuerierWrapper,
    factory_address: &Addr,
) -> StdResult<FactoryConfigResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_address.to_string(),
        msg: to_binary(&FactoryQueryMsg::Config {})?,
    }))
}

/// ## Description
/// Queries the cluster factory contract for the collector contract address
/// and the current fee rate.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **factory_address** is a reference to an object of type [`Addr`].
pub fn query_collector_contract_address(
    querier: &QuerierWrapper,
    factory_address: &Addr,
) -> StdResult<(String, String)> {
    let res = query_factory_config(&querier, &factory_address)?;
    Ok((res.commission_collector, res.protocol_fee_rate))
}

/// ## Description
/// Queries the cluster factory contract for the base denom
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **factory_address** is a reference to an object of type [`Addr`].
pub fn query_base_denom(querier: &QuerierWrapper, factory_address: &Addr) -> StdResult<String> {
    Ok(query_factory_config(&querier, &factory_address)?.base_denom)
}

/// ## Description
/// Queries the penalty contract for the amount to mint.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **penalty_address** is a reference to an object of type [`Addr`].
///
/// - **block_height** is an object of type [`u64`].
///
/// - **cluster_token_supply** is an object of type [`Uint128`].
///
/// - **inventory** is an object of type [`Vec<Uint128>`].
///
/// - **create_asset_amounts** is an object of type [`Vec<Uint128>`].
///
/// - **asset_prices** is an object of type [`Vec<String>`].
///
/// - **target_weights** is an object of type [`Vec<Uint128>`].
#[allow(clippy::too_many_arguments)]
pub fn query_create_amount(
    querier: &QuerierWrapper,
    penalty_address: &Addr,
    block_height: u64,
    cluster_token_supply: Uint128,
    inventory: Vec<Uint128>,
    create_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<Uint128>,
) -> StdResult<PenaltyCreateResponse> {
    let res: PenaltyCreateResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.to_string(),
        msg: to_binary(&PenaltyQueryMsg::PenaltyQueryCreate {
            block_height,
            cluster_token_supply,
            inventory,
            create_asset_amounts,
            asset_prices,
            target_weights,
        })?,
    }))?;

    Ok(res)
}

/// ## Description
/// Queries the penalty contract for the amount to redeem.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **penalty_address** is a reference to an object of type [`Addr`].
///
/// - **block_height** is an object of type [`u64`].
///
/// - **cluster_token_supply** is an object of type [`Uint128`].
///
/// - **inventory** is an object of type [`Vec<Uint128>`].
///
/// - **max_tokens** is an object of type [`Uint128`].
///
/// - **redeem_asset_amounts** is an object of type [`Vec<Uint128>`].
///
/// - **asset_prices** is an object of type [`Vec<String>`].
///
/// - **target_weights** is an object of type [`Vec<Uint128>`].
#[allow(clippy::too_many_arguments)]
pub fn query_redeem_amount(
    querier: &QuerierWrapper,
    penalty_address: &Addr,
    block_height: u64,
    cluster_token_supply: Uint128,
    inventory: Vec<Uint128>,
    max_tokens: Uint128,
    redeem_asset_amounts: Vec<Uint128>,
    asset_prices: Vec<String>,
    target_weights: Vec<Uint128>,
) -> StdResult<PenaltyRedeemResponse> {
    let res: PenaltyRedeemResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: penalty_address.to_string(),
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
