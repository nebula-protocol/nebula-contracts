#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response,
    StdResult, WasmQuery,
};

use crate::state::{read_config, store_config, Config};

use crate::error::ContractError;
use astroport::asset::AssetInfo;
use cw2::set_contract_version;
use nebula_protocol::oracle::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, PriceResponse, QueryMsg,
};
use tefi_oracle::hub::{
    HubQueryMsg as TeFiOracleQueryMsg, PriceResponse as TeFiOraclePriceResponse,
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-oracle";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        // Validate address format
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        oracle_addr: deps.api.addr_validate(msg.oracle_addr.as_str())?,
        base_denom: msg.base_denom,
    };

    store_config(deps.storage, &cfg)?;

    let log = vec![attr("owner", msg.owner)];

    Ok(Response::new().add_attributes(log))
}

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             oracle_addr,
///             base_denom,
///         }** Updates general oracle contract parameters.
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

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is the contract owner.
///
/// - **oracle_addr** is an object of type [`Option<String>`] which is an address
///     of a TeFi oracle hub contract.
///
/// - **base_denom** is an object of type [`Option<String>`] which is a base denom, UST.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    oracle_addr: Option<String>,
    base_denom: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // Validate address format
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    if let Some(oracle_addr) = oracle_addr {
        // Validate address format
        config.oracle_addr = deps.api.addr_validate(oracle_addr.as_str())?;
    }

    if let Some(base_denom) = base_denom {
        config.base_denom = base_denom;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Config {}** Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// - **QueryMsg::Price {
///             base_asset,
///             quote_asset,
///         }** Returns the latest oracle price of `base_asset` in `quote_asset` unit.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Price {
            base_asset,
            quote_asset,
        } => to_binary(&query_price(deps, base_asset, quote_asset)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        oracle_addr: state.oracle_addr.to_string(),
        base_denom: state.base_denom,
    };

    Ok(resp)
}

/// ## Description
/// Returns the latest oracle price of `base_asset` in `quote_asset` unit.
/// -- `latest_base_price`/`latest_quote_price`
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **base_asset** is an object of type [`AssetInfo`] which is an asset to be queried.
///
/// - **quote_asset** is an object of type [`AssetInfo`] which is an asset used as
///     a price unit.
fn query_price(
    deps: Deps,
    base_asset: AssetInfo,
    quote_asset: AssetInfo,
) -> StdResult<PriceResponse> {
    // Get latest price of `base_asset`
    let (price_base, last_updated_base) = query_asset_price(deps, base_asset)?;
    // Get latest price of `quote_asset`
    let (price_quote, last_updated_quote) = query_asset_price(deps, quote_asset)?;

    // Compute the price
    // -- rate = price_base / price_quote
    let rate = price_base / price_quote;

    Ok(PriceResponse {
        rate,
        last_updated_base,
        last_updated_quote,
    })
}

/// ## Description
/// Returns the latest price of an asset in TODO.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **asset** is an object of type [`AssetInfo`] which is the asset to be queried for its price.
fn query_asset_price(deps: Deps, asset: AssetInfo) -> StdResult<(Decimal, u64)> {
    let config: Config = read_config(deps.storage)?;

    let asset_token = match asset {
        // If native, query with denom
        AssetInfo::NativeToken { denom } => denom,
        // Otherwise, query with cw20 address
        AssetInfo::Token { contract_addr } => contract_addr.to_string(),
    };

    let res: TeFiOraclePriceResponse =
    // Get the price of a CW20 asset in TODO (from TeFi oracle hub contract)
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.oracle_addr.to_string(),
        msg: to_binary(&TeFiOracleQueryMsg::Price {
            asset_token,
            timeframe: None,
        })
        .unwrap(),
    }))?;

    Ok((res.rate, res.last_updated))
}

/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
