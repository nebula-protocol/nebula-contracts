use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult,
    Storage, Uint128,
};

use crate::msg::{HandleMsg, InitMsg, MintResponse, ParamsResponse, QueryMsg, RedeemResponse};
use crate::state::{read_config, save_config, PenaltyConfig, PenaltyParams};
use basket_math::{abs, add, div_const, dot, mul, mul_const, sub, sum, FPDecimal};
use std::cmp::{max, min};
use std::str::FromStr;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let cfg = PenaltyConfig {
        penalty_params: msg.penalty_params.clone(),
    };
    save_config(&mut deps.storage, &cfg)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: HandleMsg,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn int32_vec_to_fpdec(arr: &Vec<u32>) -> Vec<FPDecimal> {
    arr.iter()
        .map(|val| FPDecimal::from(*val as u128))
        .collect()
}

pub fn int_vec_to_fpdec(arr: &Vec<Uint128>) -> Vec<FPDecimal> {
    arr.iter().map(|val| FPDecimal::from(val.u128())).collect()
}

pub fn str_vec_to_fpdec(arr: &Vec<String>) -> StdResult<Vec<FPDecimal>> {
    arr.iter()
        .map(|val| FPDecimal::from_str(val))
        .collect::<StdResult<Vec<FPDecimal>>>()
}

pub fn imbalance(i: &Vec<FPDecimal>, p: &Vec<FPDecimal>, w: &Vec<FPDecimal>) -> FPDecimal {
    let u = div_const(&mul(w, p), dot(w, p));
    let err_portfolio = sub(&mul_const(&u, dot(i, p)), &mul(i, p));

    sum(&abs(&err_portfolio))
}

pub fn notional_penalty<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    i0: &Vec<FPDecimal>,
    i1: &Vec<FPDecimal>,
    w: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
) -> StdResult<FPDecimal> {
    let cfg = read_config(&deps.storage)?;

    let imb0 = imbalance(&i0, &p, &w);
    let imb1 = imbalance(&i1, &p, &w);

    // for now just use the value of the original basket as e...
    let e = dot(&i0, &p);

    let PenaltyParams {
        penalty_amt_lo,
        penalty_cutoff_lo,
        penalty_amt_hi,
        penalty_cutoff_hi,
        reward_amt,
        reward_cutoff,
    } = cfg.penalty_params;

    return if imb0 > imb1 {
        // use penalty function
        let cutoff_lo = penalty_cutoff_lo * e;
        let cutoff_hi = penalty_cutoff_hi * e;

        // penalty function is broken into three pieces, where its flat, linear, and then flat
        // compute the area under each piece separately

        let penalty_1 = (min(imb0, cutoff_lo) - min(imb1, cutoff_lo)) * penalty_amt_lo;

        // clip to only middle portion
        let imb0_mid = min(max(imb1, cutoff_lo), cutoff_hi);
        let imb1_mid = min(max(imb1, cutoff_lo), cutoff_hi);

        let amt_gap = penalty_amt_hi - penalty_amt_lo;
        let cutoff_gap = cutoff_hi - cutoff_lo;

        // value of y when x is at imb0_mid and imb1_mid respectively
        let imb0_mid_height = (imb0_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;
        let imb1_mid_height = (imb1_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;

        // area of a trapezoid
        let penalty_2 = (imb0_mid_height + imb1_mid_height) * (imb0_mid - imb1_mid).div(2);

        let penalty_3 = (max(imb1, cutoff_hi) - max(imb1, cutoff_hi)) * penalty_amt_hi;
        Ok(FPDecimal::zero() - (penalty_1 + penalty_2 + penalty_3))
    } else {
        // use reward function
        let cutoff = reward_cutoff * e;
        Ok((max(imb0, cutoff) - max(imb1, cutoff)) * reward_amt)
    };
}

pub fn compute_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<MintResponse> {
    let n = FPDecimal::from(basket_token_supply.u128());
    let i0 = int_vec_to_fpdec(inventory);
    let c = int_vec_to_fpdec(mint_asset_amounts);
    let p = str_vec_to_fpdec(asset_prices)?;
    let w = int32_vec_to_fpdec(target_weights);

    let i1 = add(&i0, &c);

    let penalty = notional_penalty(&deps, &i0, &i1, &w, &p)?;
    let notional_value = dot(&c, &p) + penalty;

    let mint_subtotal = n * notional_value / dot(&i0, &p);

    Ok(MintResponse {
        mint_tokens: Uint128(mint_subtotal.into()),
        log: vec![log("penalty", penalty)],
    })
}

pub fn compute_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    max_tokens: &Uint128,
    redeem_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<RedeemResponse> {
    let n = FPDecimal::from(basket_token_supply.u128());
    let i0 = int_vec_to_fpdec(inventory);
    let m = FPDecimal::from(max_tokens.u128());
    let r = int_vec_to_fpdec(redeem_asset_amounts);
    let p = str_vec_to_fpdec(asset_prices)?;
    let w = int32_vec_to_fpdec(target_weights);

    return if redeem_asset_amounts.is_empty() {
        // pro-rata redeem
        let redeem_arr = div_const(&mul_const(&w, m * dot(&i0, &p)), n * dot(&w, &p));
        Ok(RedeemResponse {
            token_cost: Uint128(m.into()),
            redeem_assets: redeem_arr
                .iter()
                .map(|&x| Uint128(x.into()))
                .collect::<Vec<Uint128>>(),
            log: vec![],
        })
    } else {
        let i1 = sub(&i0, &r);

        let penalty = notional_penalty(&deps, &i0, &i1, &w, &p)?;
        let notional_value = dot(&r, &p) - penalty;

        let needed_tokens = n * notional_value / dot(&i0, &p);

        let mut token_cost = needed_tokens.into();
        if needed_tokens != FPDecimal::from(token_cost) {
            token_cost += 1;
        }

        Ok(RedeemResponse {
            token_cost: Uint128(token_cost),
            redeem_assets: r
                .iter()
                .map(|&x| Uint128(x.into()))
                .collect::<Vec<Uint128>>(),
            log: vec![log("penalty", penalty)],
        })
    };
}

pub fn get_params<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ParamsResponse> {
    let cfg = read_config(&deps.storage)?;
    Ok(ParamsResponse {
        penalty_params: cfg.penalty_params,
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
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_redeem(
            deps,
            &basket_token_supply,
            &inventory,
            &max_tokens,
            &redeem_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::Params {} => to_binary(&get_params(deps)?),
    }
}
