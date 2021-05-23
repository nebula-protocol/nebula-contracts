use crate::error;
use nebula_protocol::cluster::{InitMsg, BasketConfig};
use crate::{
    state::{save_config, save_target_asset_data, TargetAssetData},
    util::vec_to_string,
};
use cosmwasm_std::{log, Api, Env, Extern, InitResponse, Querier, StdResult, Storage, CosmosMsg, WasmMsg};

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

    let cfg = BasketConfig {
        name: msg.name.clone(),
        owner: msg.owner.clone(),
        basket_token: msg.basket_token,
        factory: msg.factory,
        pricing_oracle: msg.pricing_oracle.clone(),
        composition_oracle: msg.composition_oracle.clone(),
        penalty: msg.penalty.clone(),
    };

    // TODO: See if we need clone here
    let assets = msg.assets.clone();
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
            messages: vec![]
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
        assert_eq!(vec![20, 10, 65, 5], value.target);
    }
}
