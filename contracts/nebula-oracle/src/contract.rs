#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmQuery,
};

use crate::state::{read_config, store_config, Config};

use crate::error::ContractError;
use astroport::asset::AssetInfo;
use nebula_protocol::oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use tefi_oracle::hub::{
    HubQueryMsg as TeFiOracleQueryMsg, PriceResponse as TeFiOraclePriceResponse,
};
use terra_cosmwasm::{ExchangeRatesResponse, TerraQuerier};

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let cfg = Config {
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        oracle_addr: deps.api.addr_validate(msg.oracle_addr.as_str())?,
        base_denom: msg.base_denom,
    };

    store_config(deps.storage, &cfg)?;

    let log = vec![attr("owner", msg.owner)];

    Ok(Response::new().add_attributes(log))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            oracle_addr,
            base_denom,
        } => update_config(deps, info, owner, oracle_addr, base_denom),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    oracle_addr: Option<String>,
    base_denom: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    if let Some(oracle_addr) = oracle_addr {
        config.oracle_addr = deps.api.addr_validate(oracle_addr.as_str())?;
    }

    if let Some(base_denom) = base_denom {
        config.base_denom = base_denom;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price {
            base_asset,
            quote_asset,
        } => to_binary(&query_price(deps, base_asset, quote_asset)?),
    }
}

fn query_price(
    deps: Deps,
    base_asset: AssetInfo,
    quote_asset: AssetInfo,
) -> StdResult<PriceResponse> {
    let (price_base, last_updated_base) = query_asset_price(deps, base_asset)?;
    let (price_quote, last_updated_quote) = query_asset_price(deps, quote_asset)?;

    let rate = Decimal::from_ratio(
        price_base * DECIMAL_FRACTIONAL,
        price_quote * DECIMAL_FRACTIONAL,
    );

    Ok(PriceResponse {
        rate,
        last_updated_base,
        last_updated_quote,
    })
}

fn query_asset_price(deps: Deps, asset: AssetInfo) -> StdResult<(Decimal, u64)> {
    let config: Config = read_config(deps.storage)?;

    match asset {
        AssetInfo::NativeToken { denom } => query_native_price(deps, denom, &config),
        AssetInfo::Token { contract_addr } => query_cw20_price(deps, contract_addr, &config),
    }
}

fn query_native_price(deps: Deps, denom: String, config: &Config) -> StdResult<(Decimal, u64)> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let res: ExchangeRatesResponse =
        terra_querier.query_exchange_rates(denom, vec![config.base_denom.clone()])?;

    Ok((res.exchange_rates[0].exchange_rate, u64::MAX))
}

fn query_cw20_price(deps: Deps, contract_addr: Addr, config: &Config) -> StdResult<(Decimal, u64)> {
    let res: TeFiOraclePriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.oracle_addr.to_string(),
            msg: to_binary(&TeFiOracleQueryMsg::Price {
                asset_token: contract_addr.to_string(),
                timeframe: None,
            })
            .unwrap(),
        }))?;

    Ok((res.rate, res.last_updated))
}
