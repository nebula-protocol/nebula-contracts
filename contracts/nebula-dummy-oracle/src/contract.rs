#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{
    config_store, read_config, read_last_update_time, read_price, save_config, set_price,
    store_last_update_time, Config,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        owner: deps.api.addr_canonicalize(&msg.owner)?,
    };

    save_config(deps.storage, &cfg)?;

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
    let api = deps.api;
    config_store(deps.storage).update(|mut config| {
        if config.owner != api.addr_canonicalize(info.sender.as_str())? {
            return Err(StdError::generic_err("unauthorized"));
        }

        if let Some(owner) = owner {
            config.owner = api.addr_canonicalize(&owner)?;
        }

        Ok(config)
    })?;

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
    if deps.api.addr_canonicalize(info.sender.as_str())? != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    for (asset, price) in prices.iter() {
        set_price(deps.storage, asset, price)?;
    }
    store_last_update_time(deps.storage, &env.block.time.seconds())?;

    let log = vec![
        attr("action", "try_set_prices"),
        attr("update_time", env.block.time.seconds().to_string()),
    ];

    Ok(Response::new().add_attributes(log))
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
