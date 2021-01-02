use cosmwasm_std::{
    from_binary, log, Api, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier, StdError,
    StdResult, Storage, Uint128,
};

use cw20::Cw20ReceiveMsg;

use super::load_ext::{load_balance, load_price};
use crate::msg::{Cw20HookMsg, HandleMsg};
use crate::penalty::{compute_penalty, compute_score};
use crate::state::{read_config, read_target, PenaltyParams};
use crate::util::to_fpdec_vec;
use basket_math::{dot, sum, FPDecimal};

use std::str::FromStr;

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
        HandleMsg::UnstageAsset { amount, asset } => try_unstage_asset(deps, env, asset, amount),
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
        Err(StdError::generic_err(
            "Receive Hook - missing expected .msg in body",
        ))
    }
}

pub fn try_receive_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    num_tokens: Uint128,
    asset_weights: Option<Vec<u32>>,
) -> StdResult<HandleResponse> {
    // only callable from Basket Token
    let cfg = read_config(&deps.storage)?;
    if cfg.basket_token != deps.api.canonical_address(&env.message.sender)? {
        return Err(StdError::unauthorized());
    }

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

/// May be called by the Basket contract owner to reset the target
pub fn try_reset_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    target: Vec<u32>,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
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
    let inv = to_fpdec_vec(&inventory);
    let c = to_fpdec_vec(&asset_amounts);

    // get current prices of each token via oracle
    let prices: Vec<Uint128> = cfg
        .assets
        .iter()
        .map(|asset| load_price(&cfg.oracle, &asset).unwrap())
        .collect();
    let p = to_fpdec_vec(&prices);

    // compute penalty
    let score = compute_score(&inv, &c, &target, &p);
    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);

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

pub fn try_unstage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    asset: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

#[cfg(test)]
mod tests {

    use crate::test_helper::prelude::*;
    #[test]
    fn mint() {}
}
