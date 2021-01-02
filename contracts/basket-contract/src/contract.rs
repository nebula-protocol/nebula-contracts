use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::msg::{Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, TargetResponse};
use crate::state::{read_config, read_target, save_config, save_target, BasketConfig};

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
        owner: deps.api.canonical_address(&msg.owner)?,
        basket_token: deps.api.canonical_address(&msg.basket_token)?,
        oracle: deps.api.canonical_address(&msg.oracle)?,
        assets,
        penalty_params: msg.penalty_params,
    };

    save_config(&mut deps.storage, &cfg)?;
    save_target(&mut deps.storage, &msg.target)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::Mint {
            asset_amounts,
            min_tokens,
        } => try_mint(deps, env, asset_amounts, min_tokens),
        HandleMsg::ResetTarget { target } => try_reset_target(deps, env, target),
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::Burn {
                num_tokens,
                asset_weights,
            } => try_receive_burn(deps, env, num_tokens, asset_weights),
            Cw20HookMsg::StageAsset { asset, amount } => {
                try_receive_stage_asset(deps, env, asset, amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

pub fn try_receive_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    num_tokens: Uint128,
    asset_weights: Option<Vec<u32>>,
) -> StdResult<HandleResponse> {
    // TODO: implement burn
    Ok(HandleResponse::default())
}

pub fn try_receive_stage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current balance of account
pub fn load_balance(
    asset_address: &CanonicalAddr,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    Ok(Uint128::zero())
}

/// EXTERNAL QUERY
/// -- Queries the oracle contract for the current asset price
pub fn load_price(
    oracle_address: &CanonicalAddr,
    asset_address: &CanonicalAddr,
) -> StdResult<Uint128> {
    Ok(Uint128::zero())
}

pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    asset_amounts: Vec<Uint128>,
    min_tokens: Option<Uint128>,
) -> StdResult<HandleResponse> {
    // get current balances of each token (inventory)
    let cfg = read_config(&deps.storage)?;
    let target = read_target(&deps.storage)?;
    let inventory: Vec<Uint128> = cfg
        .assets
        .iter()
        .map(|asset| load_balance(&asset, &env.contract.address).unwrap())
        .collect();
    let inv = to_fixed_vec(&inventory);
    let c = to_fixed_vec(&asset_amounts);

    // get current prices of each token via oracle
    let prices: Vec<Uint128> = cfg
        .assets
        .iter()
        .map(|asset| load_price(&cfg.oracle, &asset).unwrap())
        .collect();
    let p = to_fixed_vec(&prices);

    // compute penalty
    let score = compute_score(&inv, &c, &target, &p);
    let penalty = compute_penalty(&score, &cfg.penalty_params);

    // computer number of new tokens
    let new_minted = penalty * dot(&c, &p) / dot(&inv, &p);

    if let Some(minimum) = min_tokens {
        if new_minted.0 < minimum.u128() as i128 {
            return Err(StdError::generic_err(format!(
                "minted tokens {} less than minimum {}",
                new_minted.0, minimum
            )));
        }
    }

    // mint and send number of tokens to user
    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("score", score),
            log("penalty", penalty),
            log("new_minted", new_minted),
        ],
        data: None,
    })
}

/// May be called by the Basket contract owner to reset the target
pub fn try_reset_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    target: Vec<u32>,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTarget {} => to_binary(&query_target(deps)),
    }
}

fn query_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TargetResponse> {
    let target = read_target(&deps.storage)?;
    Ok(TargetResponse { target })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    // #[test]
    // fn proper_initialization() {
    //     let mut deps = mock_dependencies(20, &[]);

    //     let msg = InitMsg { count: 17 };
    //     let env = mock_env("creator", &coins(1000, "earth"));

    //     // we can just call .unwrap() to assert this was a success
    //     let res = init(&mut deps, env, msg).unwrap();
    //     assert_eq!(0, res.messages.len());

    //     // it worked, let's query the state
    //     let res = query(&deps, QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(17, value.count);
    // }

    // #[test]
    // fn increment() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));

    //     let msg = InitMsg { count: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();

    //     // beneficiary can release it
    //     let env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Increment {};
    //     let _res = handle(&mut deps, env, msg).unwrap();

    //     // should increase counter by 1
    //     let res = query(&deps, QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(18, value.count);
    // }

    // #[test]
    // fn reset() {
    //     let mut deps = mock_dependencies(20, &coins(2, "token"));

    //     let msg = InitMsg { count: 17 };
    //     let env = mock_env("creator", &coins(2, "token"));
    //     let _res = init(&mut deps, env, msg).unwrap();

    //     // beneficiary can release it
    //     let unauth_env = mock_env("anyone", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let res = handle(&mut deps, unauth_env, msg);
    //     match res {
    //         Err(StdError::Unauthorized { .. }) => {}
    //         _ => panic!("Must return unauthorized error"),
    //     }

    //     // only the original creator can reset the counter
    //     let auth_env = mock_env("creator", &coins(2, "token"));
    //     let msg = HandleMsg::Reset { count: 5 };
    //     let _res = handle(&mut deps, auth_env, msg).unwrap();

    //     // should now be 5
    //     let res = query(&deps, QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(5, value.count);
    // }
}
