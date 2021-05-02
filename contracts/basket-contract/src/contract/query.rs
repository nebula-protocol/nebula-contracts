use cosmwasm_std::{
    to_binary, Api, Binary, Extern, HumanAddr, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::ext_query::{query_cw20_balance_minus_staged, query_cw20_token_supply, query_price, query_native_balance_minus_staged};
use crate::msg::{
    BasketStateResponse, ConfigResponse, QueryMsg, StagedAmountResponse, TargetResponse,
};
use crate::state::{read_config, read_staged_asset, read_target_asset_data};
use terraswap::asset::AssetInfo;

/// Convenience function for creating inline HumanAddr
pub fn h(s: &str) -> HumanAddr {
    HumanAddr(s.to_string())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Target {} => to_binary(&query_target(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::StagedAmount { account, asset } => {
            to_binary(&query_staged_amount(deps, &account, &asset)?)
        }
        QueryMsg::BasketState {
            basket_contract_address,
        } => to_binary(&query_basket_state(deps, &basket_contract_address)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let cfg = read_config(&deps.storage)?;
    Ok(ConfigResponse { config: cfg })
}

fn query_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TargetResponse> {
    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let target = target_asset_data
        .iter()
        .map(|x| x.target)
        .collect::<Vec<_>>();
    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    Ok(TargetResponse { assets, target })
}

fn query_staged_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    asset: &AssetInfo,
) -> StdResult<StagedAmountResponse> {
    let staged_amount = read_staged_asset(&deps.storage, account, asset)?;
    Ok(StagedAmountResponse { staged_amount })
}

pub fn query_basket_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_contract_address: &HumanAddr,
) -> StdResult<BasketStateResponse> {
    let cfg = &read_config(&deps.storage)?;

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let penalty: HumanAddr = HumanAddr::from(&cfg.penalty);

    let basket_token = &cfg
        .basket_token
        .clone()
        .ok_or_else(|| StdError::generic_err("no basket token exists"))?;

    // get supply from basket token
    let outstanding_balance_tokens = query_cw20_token_supply(&deps, basket_token)?;

    // get prices for each asset
    let prices = assets
        .iter()
        .map(|asset_info| query_price(&deps, &cfg.oracle, asset_info))
        .collect::<StdResult<Vec<FPDecimal>>>()?;

    // get inventory
    let inv: Vec<Uint128> = assets
        .iter()
        .map(|asset| match asset {
            AssetInfo::Token { contract_addr } => {
                query_cw20_balance_minus_staged(&deps, &contract_addr, basket_contract_address)
            }
            AssetInfo::NativeToken { denom } => {
                query_native_balance_minus_staged(&deps, denom, basket_contract_address)
            }
        })
        .collect::<StdResult<Vec<Uint128>>>()?;

    let target_asset_data = read_target_asset_data(&deps.storage)?;

    let target = target_asset_data
        .iter()
        .map(|x| x.target)
        .collect::<Vec<_>>();

    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    Ok(BasketStateResponse {
        outstanding_balance_tokens,
        prices,
        inv,
        assets,
        target,
        penalty,
        basket_contract_address: basket_contract_address.clone(),
    })
}
