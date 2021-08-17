use cosmwasm_std::{
    to_binary, Binary, Decimal, Env, Deps, DepsMut, Response, 
    QueryRequest, StdResult, WasmQuery, MessageInfo, entry_point
};

use terraswap::asset::{AssetInfo, PairInfo};
use terraswap::pair::QueryMsg as PairQueryMsg;
use terraswap::querier::query_pair_info;

use crate::msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{read_config, set_config, Config};
use terraswap::pair::PoolResponse;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        terraswap_factory: msg.terraswap_factory,
        base_denom: msg.base_denom,
    };

    set_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { base_asset, .. } => to_binary(&query_price(deps, base_asset)?),
    }
}

fn query_price(
    deps: Deps,
    asset: String,
) -> StdResult<PriceResponse> {
    let cfg = read_config(deps.storage)?;

    let asset_info = if deps
        .api
        .canonical_address(&(asset.as_str()))
        .is_ok()
    {
        AssetInfo::Token {
            contract_addr: (asset),
        }
    } else {
        AssetInfo::NativeToken { denom: asset }
    };

    let pair_info: PairInfo = query_pair_info(
        &deps,
        &cfg.terraswap_factory,
        &[
            AssetInfo::NativeToken {
                denom: cfg.base_denom.to_string(),
            },
            asset_info,
        ],
    )?;

    let pool_info: PoolResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_info.contract_addr.clone(),
        msg: to_binary(&PairQueryMsg::Pool {})?,
    }))?;

    let assets = if pool_info.assets[0].info.to_string() == cfg.base_denom {
        pool_info.assets.to_vec()
    } else {
        vec![pool_info.assets[1].clone(), pool_info.assets[0].clone()]
    };

    let rate = Decimal::from_ratio(assets[0].amount.u128(), assets[1].amount.u128());

    Ok(PriceResponse {
        rate,
        last_updated_base: u64::MAX,
        last_updated_quote: u64::MAX,
    })
}