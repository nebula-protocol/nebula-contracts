use crate::ext_query::query_asset_balance;
use crate::{
    state::{save_config, save_target_asset_data},
    util::vec_to_string,
};
use cosmwasm_std::{
    log, Api, CosmosMsg, Env, Extern, InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use nebula_protocol::cluster::{ClusterConfig, InitMsg};
use terraswap::asset::AssetInfo;

pub fn validate_targets<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
    target_assets: Vec<AssetInfo>,
    query: Option<bool>,
) -> StdResult<bool> {
    for i in 0..target_assets.len() - 1 {
        println!("what the fuck {}", target_assets[i]);
        let to_query = if query.is_some() { query.unwrap() } else { false };

        if to_query {
            println!("we querying");
            query_asset_balance(&deps.querier, &env.contract.address, &target_assets[i])?;
        }
        for j in i + 1..target_assets.len() {
            if target_assets[i].equal(&target_assets[j]) {
                return Ok(false);
            }
        }
    }
    return Ok(true);
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
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

    println!("fuck {:?}", asset_infos);

    if validate_targets(&deps, &env, asset_infos.clone(), Some(true)).is_err() {
        return Err(StdError::generic_err(
            "Cluster must contain valid assets and cannot contain duplicate assets",
        ));
    }

    let asset_data = msg.target.clone();

    save_config(&mut deps.storage, &cfg)?;
    save_target_asset_data(&mut deps.storage, &asset_data)?;

    let log = vec![
        log("name", msg.name),
        log("owner", msg.owner),
        log("assets", vec_to_string(&asset_infos)),
    ];

    if let Some(hook) = msg.init_hook {
        Ok(InitResponse {
            log,
            messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: hook.contract_addr,
                msg: hook.msg,
                send: vec![],
            })],
        })
    } else {
        Ok(InitResponse {
            log,
            messages: vec![],
        })
    }
}
