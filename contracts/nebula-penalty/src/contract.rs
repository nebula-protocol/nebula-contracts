use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use crate::state::{config_store, read_config, save_config, PenaltyConfig};
use cluster_math::{
    add, div_const, dot, imbalance, int_vec_to_fpdec, mul_const, str_vec_to_fpdec, sub, FPDecimal,
};
use nebula_protocol::penalty::{
    ExecuteMsg, InstantiateMsg, MintResponse, ParamsResponse, PenaltyParams, QueryMsg,
    RedeemResponse,
};
use std::cmp::{max, min};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    if msg.penalty_params.penalty_amt_hi != FPDecimal::one() {
        return Err(StdError::generic_err("penalty amount must reach one"));
    }

    // TODO: add logic that checks if penalty params result in attackable basket

    let cfg = PenaltyConfig {
        owner: msg.owner,
        penalty_params: msg.penalty_params,

        ema: FPDecimal::zero(),

        // know to fast forward to current net asset value if last_block == 0
        last_block: 0u64,
    };
    save_config(deps.storage, &cfg)?;
    Ok(Response::default())
}

pub fn get_ema(deps: Deps, block_height: u64, net_asset_val: FPDecimal) -> StdResult<FPDecimal> {
    let cfg = read_config(deps.storage)?;
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

pub fn notional_penalty(
    deps: Deps,
    block_height: u64,
    i0: &[FPDecimal],
    i1: &[FPDecimal],
    w: &[FPDecimal],
    p: &[FPDecimal],
) -> StdResult<FPDecimal> {
    let cfg = read_config(deps.storage)?;

    let imb0 = imbalance(i0, p, w);
    let imb1 = imbalance(i1, p, w);

    // e is the minimum of the EMA and the net asset value
    // it is important to not let e exceed the nav to prevent someone
    // pumping e to "stretch" penalty_cutoff_hi and then using it to
    // duck the cluster imbalance too high issue
    let nav = dot(i0, p);
    let e = min(get_ema(deps, block_height, nav)?, nav);

    let PenaltyParams {
        penalty_amt_lo,
        penalty_cutoff_lo,
        penalty_amt_hi,
        penalty_cutoff_hi,
        reward_amt,
        reward_cutoff,
    } = cfg.penalty_params;

    if imb0 < imb1 {
        // use penalty function
        let cutoff_lo = penalty_cutoff_lo * e;
        let cutoff_hi = penalty_cutoff_hi * e;

        if imb1 > cutoff_hi {
            return Err(StdError::generic_err("cluster imbalance too high"));
        }

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
    }
}

pub fn compute_mint(
    deps: Deps,
    block_height: u64,
    cluster_token_supply: &Uint128,
    inventory: &[Uint128],
    mint_asset_amounts: &[Uint128],
    asset_prices: &[String],
    target_weights: &[Uint128],
) -> StdResult<MintResponse> {
    let n = FPDecimal::from(cluster_token_supply.u128());
    let i0 = int_vec_to_fpdec(inventory);
    let c = int_vec_to_fpdec(mint_asset_amounts);
    let p = str_vec_to_fpdec(asset_prices)?;
    let w = int_vec_to_fpdec(target_weights);

    let i1 = add(&i0, &c);

    let penalty = notional_penalty(deps, block_height, &i0, &i1, &w, &p)?;
    let notional_value = dot(&c, &p) + penalty;

    let mint_subtotal = n * notional_value / dot(&i0, &p);

    Ok(MintResponse {
        mint_tokens: Uint128::new(mint_subtotal.into()),
        penalty: Uint128::new(
            (if penalty.sign == 1 {
                penalty
            } else {
                FPDecimal::zero()
            })
            .into(),
        ),
        attributes: vec![attr("penalty", &format!("{}", penalty))],
    })
}

#[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
pub fn compute_redeem(
    deps: Deps,
    block_height: u64,
    cluster_token_supply: &Uint128,
    inventory: &[Uint128],
    max_tokens: &Uint128,
    redeem_asset_amounts: &[Uint128],
    asset_prices: &[String],
    target_weights: &[Uint128],
) -> StdResult<RedeemResponse> {
    let n = FPDecimal::from(cluster_token_supply.u128());
    let i0 = int_vec_to_fpdec(inventory);
    let m = FPDecimal::from(max_tokens.u128());
    let r = int_vec_to_fpdec(redeem_asset_amounts);
    let p = str_vec_to_fpdec(asset_prices)?;
    let w = int_vec_to_fpdec(target_weights);

    return if redeem_asset_amounts.is_empty() {
        // pro-rata redeem
        // let redeem_arr = div_const(&mul_const(&w, m * dot(&i0, &p)), n * dot(&w, &p));
        // pro-rata redeem for inventory
        let redeem_arr = div_const(&mul_const(&i0, m), n);
        Ok(RedeemResponse {
            token_cost: Uint128::new(m.into()),
            penalty: Uint128::zero(),
            redeem_assets: redeem_arr
                .iter()
                .map(|&x| Uint128::new(x.into()))
                .collect::<Vec<Uint128>>(),
            attributes: vec![],
        })
    } else {
        let i1 = sub(&i0, &r);

        let penalty = notional_penalty(deps, block_height, &i0, &i1, &w, &p)?;
        let notional_value = dot(&r, &p) - penalty;

        let needed_tokens = n * notional_value / dot(&i0, &p);

        let mut token_cost = needed_tokens.into();
        if needed_tokens != FPDecimal::from(token_cost) {
            token_cost += 1;
        }

        Ok(RedeemResponse {
            token_cost: Uint128::new(token_cost),
            penalty: Uint128::new(
                (if penalty.sign == 1 {
                    penalty
                } else {
                    FPDecimal::zero()
                })
                .into(),
            ),
            redeem_assets: r
                .iter()
                .map(|&x| Uint128::new(x.into()))
                .collect::<Vec<Uint128>>(),
            attributes: vec![attr("penalty", &format!("{}", penalty))],
        })
    };
}

pub fn get_params(deps: Deps) -> StdResult<ParamsResponse> {
    let cfg = read_config(deps.storage)?;
    Ok(ParamsResponse {
        penalty_params: cfg.penalty_params,
        last_block: cfg.last_block,
        ema: cfg.ema.to_string(),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Mint {
            block_height,
            cluster_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_mint(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &mint_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::Redeem {
            block_height,
            cluster_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_redeem(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &max_tokens,
            &redeem_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::Params {} => to_binary(&get_params(deps)?),
    }
}

pub fn update_config(
    deps: DepsMut,
    owner: Option<String>,
    penalty_params: Option<PenaltyParams>,
) -> StdResult<Response> {
    config_store(deps.storage).update(|mut config| -> StdResult<_> {
        if let Some(owner) = owner {
            config.owner = owner;
        }

        if let Some(penalty_params) = penalty_params {
            config.penalty_params = penalty_params;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn update_ema(
    deps: DepsMut,
    block_height: u64,
    net_asset_val: FPDecimal,
) -> StdResult<Response> {
    let mut cfg = read_config(deps.storage)?;
    cfg.ema = get_ema(deps.as_ref(), block_height, net_asset_val)?;
    cfg.last_block = block_height;
    save_config(deps.storage, &cfg)?;
    Ok(Response::new().add_attributes(vec![attr("new_ema", &format!("{}", cfg.ema.to_string()))]))
}

pub fn execute_mint(
    deps: DepsMut,
    block_height: u64,
    _cluster_token_supply: &Uint128,
    inventory: &Vec<Uint128>,
    _mint_asset_amounts: &Vec<Uint128>,
    asset_prices: &Vec<String>,
    _target_weights: &Vec<Uint128>,
) -> StdResult<Response> {
    let i = int_vec_to_fpdec(inventory);
    let p = str_vec_to_fpdec(asset_prices)?;
    update_ema(deps, block_height, dot(&i, &p))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_redeem(
    deps: DepsMut,
    block_height: u64,
    _cluster_token_supply: &Uint128,
    inventory: &[Uint128],
    _max_tokens: &Uint128,
    _redeem_asset_amounts: &[Uint128],
    asset_prices: &[String],
    _target_weights: &[Uint128],
) -> StdResult<Response> {
    let i = int_vec_to_fpdec(inventory);
    let p = str_vec_to_fpdec(asset_prices)?;
    update_ema(deps, block_height, dot(&i, &p))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let cfg = read_config(deps.storage)?;

    // check permission
    if info.sender.to_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            penalty_params,
        } => update_config(deps, owner, penalty_params),
        ExecuteMsg::Mint {
            block_height,
            cluster_token_supply,
            inventory,
            mint_asset_amounts,
            asset_prices,
            target_weights,
        } => execute_mint(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &mint_asset_amounts,
            &asset_prices,
            &target_weights,
        ),
        ExecuteMsg::Redeem {
            block_height,
            cluster_token_supply,
            inventory,
            max_tokens,
            redeem_asset_amounts,
            asset_prices,
            target_weights,
        } => execute_redeem(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &max_tokens,
            &redeem_asset_amounts,
            &asset_prices,
            &target_weights,
        ),
    }
}
