use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128,
};

use crate::state::{read_config, save_config, PenaltyConfig};
use basket_math::{
    add, div_const, dot, imbalance, int32_vec_to_fpdec, int_vec_to_fpdec, mul, mul_const,
    str_vec_to_fpdec, sub, sum, FPDecimal,
};
use nebula_protocol::penalty::{
    HandleMsg, InitMsg, MintResponse, ParamsResponse, PenaltyParams, QueryMsg, RedeemResponse,
};
use std::cmp::{max, min};
use std::str::FromStr;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let cfg = PenaltyConfig {
        owner: msg.owner.clone(),
        penalty_params: msg.penalty_params.clone(),

        ema: FPDecimal::zero(),

        // know to fast forward to current net asset value if last_block == 0
        last_block: 0u64,
    };
    save_config(&mut deps.storage, &cfg)?;
    Ok(InitResponse::default())
}

pub fn get_ema<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_height: u64,
    net_asset_val: FPDecimal,
) -> StdResult<FPDecimal> {
    let cfg = read_config(&deps.storage)?;
    let prev_ema = cfg.ema;
    let prev_block = cfg.last_block;
    if prev_block != 0u64 {
        let dt = FPDecimal::from((block_height - prev_block) as u128);

        // hard code one hour (600 blocks)
        let tau = FPDecimal::from(-600i128);
        let factor = FPDecimal::_exp(dt / tau);
        Ok(factor * prev_ema + (FPDecimal::one() - factor) * net_asset_val)
    } else {
        Ok(net_asset_val)
    }
}

pub fn notional_penalty<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_height: u64,
    i0: &Vec<FPDecimal>,
    i1: &Vec<FPDecimal>,
    w: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
) -> StdResult<FPDecimal> {
    let cfg = read_config(&deps.storage)?;

    let imb0 = imbalance(&i0, &p, &w);
    let imb1 = imbalance(&i1, &p, &w);

    // for now just use the value of the original basket as e...
    let e = get_ema(&deps, block_height, dot(&i0, &p))?;

    let PenaltyParams {
        penalty_amt_lo,
        penalty_cutoff_lo,
        penalty_amt_hi,
        penalty_cutoff_hi,
        reward_amt,
        reward_cutoff,
    } = cfg.penalty_params;

    return if imb0 < imb1 {
        // use penalty function
        let cutoff_lo = penalty_cutoff_lo * e;
        let cutoff_hi = penalty_cutoff_hi * e;

        // penalty function is broken into three pieces, where its flat, linear, and then flat
        // compute the area under each piece separately

        let penalty_1 = (min(imb1, cutoff_lo) - min(imb0, cutoff_lo)) * penalty_amt_lo;

        // clip to only middle portion
        let imb0_mid = min(max(imb0, cutoff_lo), cutoff_hi);
        let imb1_mid = min(max(imb1, cutoff_lo), cutoff_hi);

        let amt_gap = penalty_amt_hi - penalty_amt_lo;
        let cutoff_gap = cutoff_hi - cutoff_lo;

        // value of y when x is at imb0_mid and imb1_mid respectively
        let imb0_mid_height = (imb0_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;
        let imb1_mid_height = (imb1_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;

        // area of a trapezoid
        let penalty_2 = (imb0_mid_height + imb1_mid_height) * (imb1_mid - imb0_mid).div(2);

        let penalty_3 = (max(imb1, cutoff_hi) - max(imb0, cutoff_hi)) * penalty_amt_hi;
        Ok(FPDecimal::zero() - (penalty_1 + penalty_2 + penalty_3))
    } else {
        // use reward function
        let cutoff = reward_cutoff * e;
        Ok((max(imb0, cutoff) - max(imb1, cutoff)) * reward_amt)
    };
}

pub fn compute_mint<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_height: u64,
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

    let penalty = notional_penalty(&deps, block_height, &i0, &i1, &w, &p)?;
    let notional_value = dot(&c, &p) + penalty;

    let mint_subtotal = n * notional_value / dot(&i0, &p);

    Ok(MintResponse {
        mint_tokens: Uint128(mint_subtotal.into()),
        penalty: Uint128(
            (if penalty.sign == 1 {
                penalty
            } else {
                FPDecimal::zero()
            })
            .into(),
        ),
        log: vec![log("penalty", penalty)],
    })
}

pub fn compute_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    block_height: u64,
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
            penalty: Uint128::zero(),
            redeem_assets: redeem_arr
                .iter()
                .map(|&x| Uint128(x.into()))
                .collect::<Vec<Uint128>>(),
            log: vec![],
        })
    } else {
        let i1 = sub(&i0, &r);

        let penalty = notional_penalty(&deps, block_height, &i0, &i1, &w, &p)?;
        let notional_value = dot(&r, &p) - penalty;

        let needed_tokens = n * notional_value / dot(&i0, &p);

        let mut token_cost = needed_tokens.into();
        if needed_tokens != FPDecimal::from(token_cost) {
            token_cost += 1;
        }

        Ok(RedeemResponse {
            token_cost: Uint128(token_cost),
            penalty: Uint128(
                (if penalty.sign == 1 {
                    penalty
                } else {
                    FPDecimal::zero()
                })
                .into(),
            ),
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
            block_height,
            basket_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_mint(
            deps,
            block_height,
            &basket_token_supply,
            &inventory,
            &mint_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::Redeem {
            block_height,
            basket_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_redeem(
            deps,
            block_height,
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

pub fn try_reset_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let mut new_cfg = cfg.clone();
    new_cfg.owner = owner.clone();
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "_try_reset_owner")],
        data: None,
    })
}

pub fn update_ema<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    block_height: u64,
    net_asset_val: FPDecimal,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let mut new_cfg = cfg.clone();
    new_cfg.ema = get_ema(&deps, block_height, net_asset_val)?;
    new_cfg.last_block = block_height;
    save_config(&mut deps.storage, &new_cfg)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("new_ema", new_cfg.ema)],
        data: None,
    })
}

pub fn handle_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    block_height: u64,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<HandleResponse> {
    let i = int_vec_to_fpdec(inventory);
    let p = str_vec_to_fpdec(asset_prices)?;
    update_ema(deps, block_height, dot(&i, &p))
}

pub fn handle_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    block_height: u64,
    basket_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    max_tokens: &Uint128,
    redeem_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    target_weights: &Vec<u32>,
) -> StdResult<HandleResponse> {
    let i = int_vec_to_fpdec(inventory);
    let p = str_vec_to_fpdec(asset_prices)?;
    update_ema(deps, block_height, dot(&i, &p))
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    match msg {
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, &owner),
        HandleMsg::Mint {
            block_height,
            basket_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        } => handle_mint(
            deps,
            block_height,
            &basket_token_supply,
            &inventory,
            &mint_asset_amounts,
            &asset_prices,
            &target_weights,
        ),
        HandleMsg::Redeem {
            block_height,
            basket_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        } => handle_redeem(
            deps,
            block_height,
            &basket_token_supply,
            &inventory,
            &max_tokens,
            &redeem_asset_amounts,
            &asset_prices,
            &target_weights,
        ),
    }
}
