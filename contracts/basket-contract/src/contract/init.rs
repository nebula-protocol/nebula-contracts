use crate::{error, msg::InitMsg, state::PenaltyParams};
use crate::{
    state::{save_config, save_target, save_asset_data, BasketConfig, AssetData},
    util::vec_to_string,
};
use cosmwasm_std::{log, Api, Env, Extern, InitResponse, Querier, StdResult, Storage};

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
        assets: msg.assets.clone(),
        penalty_params: msg.penalty_params,
    };


    // TODO: See if we need clone here
    let assets = msg.assets.clone();
    let target = msg.target.clone();

    // TODO: Consider renaming struct name
    let mut asset_data: Vec<AssetData> = Vec::new();
    for i in 0..msg.target.len() {
        let asset_elem = AssetData {
            asset: assets[i].clone(),
            target: target[i].clone(),
        };
        asset_data.push(asset_elem);
    }

    save_config(&mut deps.storage, &cfg)?;
    save_asset_data(&mut deps.storage, &asset_data)?;
    save_target(&mut deps.storage, &msg.target)?;

    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = msg.penalty_params;
    Ok(InitResponse {
        log: vec![
            log("name", msg.name),
            log("owner", msg.owner),
            log("assets", vec_to_string(&msg.assets)),
            log(
                "penalty_params",
                format!("({}, {}, {}, {})", a_pos, a_neg, s_pos, s_neg),
            ),
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
