use crate::{error, msg::InitMsg};
use crate::{
    state::{save_config, save_target_asset_data, BasketConfig, TargetAssetData},
    util::vec_to_string,
};
use cosmwasm_std::{log, Api, Env, Extern, InitResponse, Querier, StdResult, Storage};
use terraswap::asset::AssetInfo;

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
        oracle: msg.oracle.clone(),
        penalty: msg.penalty.clone(),
    };

    // TODO: See if we need clone here
    let assets = msg.assets.clone();
    let target = msg.target.clone();

    // TODO: Consider renaming struct name
    let mut asset_data: Vec<TargetAssetData> = Vec::new();
    for i in 0..msg.target.len() {
        let asset_elem = TargetAssetData {
            asset: AssetInfo::Token {
                contract_addr: assets[i].clone(),
            },
            target: target[i].clone(),
        };
        asset_data.push(asset_elem);
    }

    save_config(&mut deps.storage, &cfg)?;
    save_target_asset_data(&mut deps.storage, &asset_data)?;

    Ok(InitResponse {
        log: vec![
            log("name", msg.name),
            log("owner", msg.owner),
            log("assets", vec_to_string(&msg.assets)),
        ],
        messages: vec![],
    })
}

#[cfg(test)]
mod tests {
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
