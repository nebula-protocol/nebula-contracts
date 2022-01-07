#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{to_binary, Binary, Deps, Env, StdError, StdResult, Uint128};

use crate::ext_query::{query_cw20_token_supply, query_price};
use crate::state::{read_asset_balance, read_config, read_target_asset_data};
use astroport::asset::AssetInfo;
use nebula_protocol::cluster::{
    ClusterInfoResponse, ClusterStateResponse, ConfigResponse, QueryMsg, TargetResponse,
};

/// Convenience function for creating inline String
pub fn h(s: &str) -> String {
    s.to_string()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Target {} => to_binary(&query_target(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ClusterState {} => to_binary(&query_cluster_state(
            deps,
            &env.contract.address.to_string(),
            0,
        )?),
        QueryMsg::ClusterInfo {} => to_binary(&query_cluster_info(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = read_config(deps.storage)?;
    Ok(ConfigResponse { config: cfg })
}

fn query_target(deps: Deps) -> StdResult<TargetResponse> {
    let target_assets = read_target_asset_data(deps.storage)?;
    Ok(TargetResponse {
        target: target_assets,
    })
}

pub fn query_cluster_state(
    deps: Deps,
    cluster_contract_address: &String,
    stale_threshold: u64,
) -> StdResult<ClusterStateResponse> {
    let cfg = &read_config(deps.storage)?;

    let active = cfg.active;

    let target_asset_data = read_target_asset_data(deps.storage)?;
    let asset_infos = target_asset_data
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    let penalty: String = cfg.penalty.clone();

    let cluster_token = cfg
        .cluster_token
        .clone()
        .ok_or_else(|| StdError::generic_err("no cluster token exists"))?;

    // get supply from cluster token
    let outstanding_balance_tokens = query_cw20_token_supply(&deps.querier, &cluster_token)?;

    if !active && stale_threshold != u64::MIN {
        return Err(StdError::generic_err(
            "Decommissioned cluster should have int min stale threshold",
        ));
    }

    let prices = asset_infos
        .iter()
        .map(|asset_info| {
            query_price(
                &deps.querier,
                &cfg.pricing_oracle,
                asset_info,
                stale_threshold,
            )
        })
        .collect::<StdResult<Vec<String>>>()?;

    // get inventory
    let inv: Vec<Uint128> = asset_infos
        .iter()
        .map(|asset| match asset {
            AssetInfo::Token { contract_addr } => {
                read_asset_balance(deps.storage, &contract_addr.to_string())
            }
            AssetInfo::NativeToken { denom } => read_asset_balance(deps.storage, denom),
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
        active,
    })
}

pub fn query_cluster_info(deps: Deps) -> StdResult<ClusterInfoResponse> {
    let cfg = &read_config(deps.storage)?;
    let name = &cfg.name;
    let description = &cfg.description;
    Ok(ClusterInfoResponse {
        name: name.to_string(),
        description: description.to_string(),
    })
}
