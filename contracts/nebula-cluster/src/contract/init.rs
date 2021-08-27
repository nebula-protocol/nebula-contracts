#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::ext_query::query_asset_balance;
use crate::{
    state::{save_config, save_target_asset_data},
    util::vec_to_string,
};
use cosmwasm_std::{
    attr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdError, StdResult,
};
use nebula_protocol::cluster::{ClusterConfig, InstantiateMsg};
use terraswap::asset::AssetInfo;

pub fn validate_targets(
    querier: QuerierWrapper,
    env: &Env,
    target_assets: Vec<AssetInfo>,
    to_query: bool,
) -> StdResult<bool> {
    for i in 0..target_assets.len() - 1 {
        if to_query {
            query_asset_balance(
                &querier,
                &env.contract.address.to_string(),
                &target_assets[i],
            )?;
        }
        for j in i + 1..target_assets.len() {
            if target_assets[i].equal(&target_assets[j]) {
                return Ok(false);
            }
        }
    }
    return Ok(true);
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = ClusterConfig {
        name: msg.name.clone(),
        description: msg.description.clone(),
        owner: msg.owner.clone(),
        cluster_token: msg.cluster_token,
        factory: msg.factory,
        pricing_oracle: msg.pricing_oracle.clone(),
        target_oracle: msg.target_oracle.clone(),
        penalty: msg.penalty.clone(),
        active: true,
    };

    let asset_infos = msg
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    if validate_targets(deps.querier, &env, asset_infos.clone(), false).is_err() {
        return Err(StdError::generic_err(
            "Cluster must contain valid assets and cannot contain duplicate assets",
        ));
    }

    let asset_data = msg.target.clone();

    save_config(deps.storage, &cfg)?;
    save_target_asset_data(deps.storage, &asset_data)?;

    let log = vec![
        attr("name", msg.name),
        attr("owner", msg.owner),
        attr("assets", vec_to_string(&asset_infos)),
    ];

    Ok(Response::new().add_attributes(log))
}
