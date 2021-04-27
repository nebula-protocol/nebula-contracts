use cosmwasm_std::{to_binary, Api, Binary, Decimal, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, Uint128, log};

use basket_math::{FPDecimal, dot, sum, mul, mul_const, sub, add, abs};
use crate::msg::{HandleMsg, InitMsg, MintResponse, QueryMsg, RedeemResponse};
use std::str::FromStr;
use crate::state::{PenaltyConfig, save_config, PenaltyParams, read_config};
use crate::penalty::{compute_score, compute_penalty, compute_diff};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let cfg = PenaltyConfig {
        penalty_params: msg.penalty_params.clone(),
    };
    save_config(&mut deps.storage, &cfg)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn int32_vec_to_fpdec(arr: &Vec<u32>) -> Vec<FPDecimal> {
    arr.iter()
        .map(|val| FPDecimal::from(*val as u128))
        .collect()
}

pub fn int_vec_to_fpdec(arr: &Vec<Uint128>) -> Vec<FPDecimal> {
    arr.iter()
        .map(|val| FPDecimal::from(val.u128()))
        .collect()
}

pub fn str_vec_to_fpdec(arr: &Vec<String>) -> StdResult<Vec<FPDecimal>> {
    arr.iter()
        .map(|val| FPDecimal::from_str(val))
        .collect::<StdResult<Vec<FPDecimal>>>()
}

pub fn err(i: &Vec<FPDecimal>, p: &Vec<FPDecimal>, w: &Vec<FPDecimal>) -> Vec<FPDecimal> {
    let wp = dot(w, p);
    let u: Vec<FPDecimal> = mul(w, p).iter().map(|&i| i / wp).collect();
    sub(&mul_const(&u, dot(i, p)), &mul(i, p))
}

pub fn compute_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<MintResponse> {

    let cfg = read_config(&deps.storage)?;

    let n = FPDecimal::from(basket_token_supply.u128());
    let i = int_vec_to_fpdec(inventory);
    let c = int_vec_to_fpdec(mint_asset_amounts);
    let p = str_vec_to_fpdec(asset_prices)?;

    let score = compute_score(&i, &c, &p, target_weights).div(2i128);
    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);

    let mint_subtotal =
        penalty * dot(&c, &p) * n / dot(&i, &p);

    Ok(MintResponse {
        mint_tokens: Uint128(mint_subtotal.into()),
        log: vec![
            log("score", score),
            log("penalty", penalty),
        ],
    })
}

pub fn compute_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    redeem_tokens: &Uint128,
    redeem_weights: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<RedeemResponse> {

    let cfg = read_config(&deps.storage)?;

    let n = FPDecimal::from(basket_token_supply.u128());
    let i = int_vec_to_fpdec(inventory);
    let m = FPDecimal::from(redeem_tokens.u128());
    let r = int_vec_to_fpdec(redeem_weights);
    let p = str_vec_to_fpdec(asset_prices)?;
    //
    // let input_notional = m * dot(&i, &p) / n;
    //
    // let b = input_notional
    let weights_sum= sum(&r);

    let r: Vec<FPDecimal> = r // normalize weights vector
        .iter()
        .map(|&x| x / weights_sum)
        .collect();

    let b: Vec<FPDecimal> = r.iter().map(|&x| m * x * dot(&i, &p) / n / dot(&r, &p)).collect();
    let neg_b: Vec<FPDecimal> = b.iter().map(|&x| FPDecimal::from(-1i128) * x).collect();

    // compute score
    let diff = compute_diff(&i, &neg_b, &p, &target_weights);
    let score = (sum(&diff) / dot(&b, &p)).div(2i128);

    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);

    let redeem: Vec<Uint128> = b
        .iter()
        .map(|&x| Uint128((x * penalty).into()))
        .collect();

    Ok(RedeemResponse {
        redeem_assets: redeem,
        log: vec![
            log("score", score),
            log("penalty", penalty),
        ],
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Mint {
            basket_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_mint(
            deps,
            &basket_token_supply,
            &inventory,
            &mint_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::Redeem {
            basket_token_supply,
            inventory,
            redeem_tokens,
            redeem_weights,
            asset_prices,
            target_weights,
        } => to_binary(&compute_redeem(
            deps,
            &basket_token_supply,
            &inventory,
            &redeem_tokens,
            &redeem_weights,
            &asset_prices,
            &target_weights,
        )?),
    }
}
