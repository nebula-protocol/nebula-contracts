use cosmwasm_std::{
    to_binary, Api, Binary, Extern, HumanAddr, Querier, StdError, StdResult, Storage, Uint128,
};

use crate::ext_query::{query_cw20_balance, query_cw20_token_supply, query_price};
use crate::state::{read_config, read_target_asset_data};
use nebula_protocol::cluster::{ClusterStateResponse, ConfigResponse, QueryMsg, TargetResponse};
use terraswap::asset::AssetInfo;
use terraswap::querier::query_balance;

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
        QueryMsg::ClusterState {
            cluster_contract_address,
        } => to_binary(&query_cluster_state(deps, &cluster_contract_address, 0)?),
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
    let target_assets = read_target_asset_data(&deps.storage)?;
    Ok(TargetResponse { target: target_assets })
}

pub fn query_cluster_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    cluster_contract_address: &HumanAddr,
    stale_threshold: u64,
) -> StdResult<ClusterStateResponse> {
    let cfg = &read_config(&deps.storage)?;

    let active = cfg.active;

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let asset_infos = target_asset_data
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    let penalty: HumanAddr = HumanAddr::from(&cfg.penalty);

    let cluster_token = cfg
        .cluster_token
        .clone()
        .ok_or_else(|| StdError::generic_err("no cluster token exists"))?;

    // get supply from cluster token
    let outstanding_balance_tokens = query_cw20_token_supply(&deps.querier, &cluster_token)?;

    // get prices for each asset
    let prices = asset_infos
        .iter()
        .map(|asset_info| query_price(&deps.querier, &cfg.pricing_oracle, asset_info, stale_threshold))
        .collect::<StdResult<Vec<String>>>()?;

    // get inventory
    let inv: Vec<Uint128> = asset_infos
        .iter()
        .map(|asset| match asset {
            AssetInfo::Token { contract_addr } => {
                query_cw20_balance(&deps.querier, &contract_addr, cluster_contract_address)
            }
            AssetInfo::NativeToken { denom } => {
                query_balance(&deps, cluster_contract_address, denom.clone())
            }
        })
        .collect::<StdResult<Vec<Uint128>>>()?;

    Ok(ClusterStateResponse {
        outstanding_balance_tokens,
        prices,
        inv,
        target: target_asset_data,
        penalty,
        cluster_token,
        cluster_contract_address: cluster_contract_address.clone(),
        active
    })
}
