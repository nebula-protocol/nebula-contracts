use cosmwasm_std::{
    to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    QueryRequest, StdResult, Storage, WasmQuery,
};

use terraswap::asset::{AssetInfo, PairInfo};
use terraswap::pair::QueryMsg as PairQueryMsg;
use terraswap::querier::query_pair_info;

use crate::msg::{HandleMsg, InitMsg, PriceResponse, QueryMsg};
use crate::state::{read_config, set_config, Config};
use terraswap::pair::PoolResponse;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let config = Config {
        terraswap_factory: msg.terraswap_factory,
        base_denom: msg.base_denom,
    };

    set_config(&mut deps.storage, &config)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: HandleMsg,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { base_asset, .. } => to_binary(&query_price(deps, base_asset)?),
    }
}

fn query_price<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    asset: String,
) -> StdResult<PriceResponse> {
    let cfg = read_config(&deps.storage)?;

    let asset_info = if deps
        .api
        .canonical_address(&HumanAddr::from(asset.as_str()))
        .is_ok()
    {
        AssetInfo::Token {
            contract_addr: HumanAddr::from(asset),
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
