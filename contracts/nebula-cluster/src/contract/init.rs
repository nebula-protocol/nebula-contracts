use crate::error;
use crate::{
    state::{save_config, save_target_asset_data, TargetAssetData},
    util::vec_to_string,
};
use cosmwasm_std::{
    log, Api, CosmosMsg, Env, Extern, InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use nebula_protocol::cluster::{ClusterConfig, InitMsg};
use terraswap::asset::AssetInfo;

pub fn validate_targets(target_assets: Vec<AssetInfo>) -> bool {
    for i in 0..target_assets.len() - 1 {
        for j in i + 1..target_assets.len() {
            if target_assets[i].equal(&target_assets[j]) {
                return false;
            }
        }
    }
    return true;
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // make sure assets & target are same dimensions
    if msg.target.len() != msg.assets.len() {
        return Err(error::bad_weight_dimensions(
            msg.target.len(),
            msg.assets.len(),
        ));
    }

    let cfg = ClusterConfig {
        name: msg.name.clone(),
        owner: msg.owner.clone(),
        cluster_token: msg.cluster_token,
        factory: msg.factory,
        pricing_oracle: msg.pricing_oracle.clone(),
        composition_oracle: msg.composition_oracle.clone(),
        penalty: msg.penalty.clone(),
    };

    let assets = msg.assets.clone();

    if !validate_targets(assets.clone()) {
        return Err(StdError::generic_err(
            "Cluster cannot contain duplicate assets",
        ));
    }

    let target = msg.target.clone();

    // TODO: Consider renaming struct name
    let mut asset_data: Vec<TargetAssetData> = Vec::new();
    for i in 0..msg.target.len() {
        let asset_elem = TargetAssetData {
            asset: assets[i].clone(),
            target: target[i].clone(),
        };
        asset_data.push(asset_elem);
    }

    save_config(&mut deps.storage, &cfg)?;
    save_target_asset_data(&mut deps.storage, &asset_data)?;

    let log = vec![
        log("name", msg.name),
        log("owner", msg.owner),
        log("assets", vec_to_string(&msg.assets)),
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

#[cfg(test)]
mod tests {
    use nebula_protocol::cluster::{QueryMsg, TargetResponse};

    use crate::{q, test_helper::*};

    #[test]
    fn proper_initialization() {
        let (deps, init_res) = mock_init();
        assert_eq!(0, init_res.messages.len());

        // make sure target was saved
        let value = q!(&deps, TargetResponse, QueryMsg::Target {});
        assert_eq!(vec![20, 20, 20, 20], value.target);
    }
}
