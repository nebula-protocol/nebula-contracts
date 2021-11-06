#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{read_config, read_price, store_config, store_price, Config, PriceInfo};

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        owner: msg.owner.clone(),
    };

    store_config(deps.storage, &cfg)?;

    let log = vec![attr("owner", msg.owner)];

    Ok(Response::new().add_attributes(log))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetPrices { prices } => try_set_prices(deps, env, info, &prices),
        ExecuteMsg::UpdateConfig { owner } => update_config(deps, info, owner),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn try_set_prices(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    prices: &Vec<(String, Decimal)>,
) -> StdResult<Response> {
    let cfg = read_config(deps.storage)?;

    // check permission
    if info.sender.to_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    for (asset, price) in prices.iter() {
        let price = PriceInfo {
            price: price.clone(),
            last_updated_time: env.block.time.seconds(),
        };

        store_price(deps.storage, asset, &price)?;
    }

    let log = vec![
        attr("action", "try_set_prices"),
        attr("update_time", env.block.time.seconds().to_string()),
    ];

    Ok(Response::new().add_attributes(log))
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

fn query_price(deps: Deps, base_asset: String, quote_asset: String) -> StdResult<PriceResponse> {
    let price_info_base = read_price(deps.storage, &base_asset)?;
    let price_info_quote = read_price(deps.storage, &quote_asset)?;

    let rate = Decimal::from_ratio(
        price_info_base.price * DECIMAL_FRACTIONAL,
        price_info_quote.price * DECIMAL_FRACTIONAL,
    );

    Ok(PriceResponse {
        rate,
        last_updated_base: price_info_base.last_updated_time,
        last_updated_quote: price_info_quote.last_updated_time,
    })
}
