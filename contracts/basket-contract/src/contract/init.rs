use crate::state::{save_config, save_target, BasketConfig};
use crate::{msg::InitMsg, state::PenaltyParams};
use cosmwasm_std::{
    log, Api, CanonicalAddr, Env, Extern, InitResponse, Querier, StdResult, Storage,
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let assets: Vec<CanonicalAddr> = msg
        .assets
        .iter()
        .map(|x| deps.api.canonical_address(&x).unwrap())
        .collect();

    let cfg = BasketConfig {
        name: msg.name.clone(),
        owner: deps.api.canonical_address(&msg.owner)?,
        basket_token: deps.api.canonical_address(&msg.basket_token)?,
        oracle: deps.api.canonical_address(&msg.oracle)?,
        assets,
        penalty_params: msg.penalty_params,
    };

    save_config(&mut deps.storage, &cfg)?;
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
            log("basket_token", msg.basket_token),
            log("oracle", msg.oracle),
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

    use crate::{q, test_helper::prelude::*};

    #[test]
    fn proper_initialization() {
        let (deps, init_res) = init_contract();
        assert_eq!(0, init_res.messages.len());

        // make sure target was saved
        let value = q!(&deps, TargetResponse, QueryMsg::GetTarget {});
        assert_eq!(vec![1, 1, 2, 1], value.target);
    }
}
