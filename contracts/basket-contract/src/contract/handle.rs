use cosmwasm_std::{
    from_binary, log, Api, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier, StdError,
    StdResult, Storage, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::ext_query::{query_cw20_balance, query_price};
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
    if cfg.basket_token != env.message.sender {
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
        .map(|asset| query_cw20_balance(&deps, &asset, &env.contract.address).unwrap())
        .collect();
    let inv = to_fpdec_vec(&inventory);
    let c = to_fpdec_vec(&asset_amounts);

    // get current prices of each token via oracle
    let prices: Vec<FPDecimal> = cfg
        .assets
        .iter()
        .map(|asset| query_price(&deps, &cfg.oracle, &asset).unwrap())
        .collect();

    // compute penalty
    let score = compute_score(&inv, &c, &prices, &target);
    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);

    // computer number of new tokens
    let new_minted = penalty * dot(&c, &prices) / dot(&inv, &prices);

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

    use crate::test_helper::*;
    #[test]
    fn mint() {
        let (mut deps, _init_res) = mock_init();

        // Asset :: Curr. Price (UST) :: Balance (µ-unit), (+ proposed)
        // --
        // mAAPL ::  135.18   :: 20_053_159   (+ 5_239_222)
        // mGOOG :: 1780.03   :: 3_710_128
        // mMSFT ::  222.42   :: 8_281_228    (+ 2_332_111)
        // mNFLX ::  540.82   :: 24_212_221   (+ 0_222_272)

        deps.querier.with_oracle_prices(&[
            (&"uusd".to_string(), &Decimal::one()),
            (&"mAAPL".to_string(), &Decimal::from_str("1").unwrap()),
            (&"mGOOG".to_string(), &Decimal::from_str("1").unwrap()),
            (&"mMSFT".to_string(), &Decimal::from_str("1").unwrap()),
            (&"mNFLX".to_string(), &Decimal::from_str("1").unwrap()),
        ]);

        deps.querier.with_token_balances(&[
            (
                &h("mAAPL"),
                &[(&h(MOCK_CONTRACT_ADDR), &Uint128::from(1u128))],
            ),
            (
                &h("mGOOG"),
                &[(&h(MOCK_CONTRACT_ADDR), &Uint128::from(1u128))],
            ),
            (
                &h("mMSFT"),
                &[(&h(MOCK_CONTRACT_ADDR), &Uint128::from(1u128))],
            ),
            (
                &h("mNFLX"),
                &[(&h(MOCK_CONTRACT_ADDR), &Uint128::from(1u128))],
            ),
        ]);

        let msg = HandleMsg::Mint {
            asset_amounts: vec![
                Uint128::zero(),      // mAAPL
                Uint128::zero(),      // mGOOG
                Uint128::from(1u128), // mMSFT
                Uint128::zero(),      // mNFLX
            ],
            min_tokens: None,
        };

        let env = mock_env(consts::owner(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        for log in res.log.iter() {
            println!("{}: {}", log.key, log.value);
        }
        assert_eq!(1, res.messages.len());
    }
}
