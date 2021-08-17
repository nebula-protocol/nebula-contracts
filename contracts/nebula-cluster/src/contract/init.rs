use crate::ext_query::query_asset_balance;
use crate::{
    state::{save_config, save_target_asset_data},
    util::vec_to_string,
};
use cosmwasm_std::{
    attr, entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use nebula_protocol::cluster::{ClusterConfig, InstantiateMsg};
use terraswap::asset::AssetInfo;

pub fn validate_targets(
    deps: Deps,
    env: &Env,
    target_assets: Vec<AssetInfo>,
    query: Option<bool>,
) -> StdResult<bool> {
    for i in 0..target_assets.len() - 1 {
        let to_query = query.unwrap_or(true);
        if to_query {
            query_asset_balance(
                &deps.querier,
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
        composition_oracle: msg.composition_oracle.clone(),
        penalty: msg.penalty.clone(),
        active: true,
    };

    let asset_infos = msg
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    if validate_targets(deps.as_ref(), &env, asset_infos.clone(), Some(false)).is_err() {
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
