#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use crate::error::ContractError;
use crate::state::{config_store, read_config, store_config, PenaltyConfig};
use cluster_math::{
    add, div_const, dot, imbalance, int_vec_to_fpdec, mul_const, str_vec_to_fpdec, sub, FPDecimal,
};
use cw2::set_contract_version;
use nebula_protocol::penalty::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, ParamsResponse, PenaltyCreateResponse,
    PenaltyParams, PenaltyRedeemResponse, QueryMsg,
};
use std::cmp::{max, min};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-penalty";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.penalty_params.penalty_amt_hi != FPDecimal::one() {
        return Err(ContractError::Generic(
            "penalty amount must reach one".to_string(),
        ));
    }

    let cfg = PenaltyConfig {
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        penalty_params: msg.penalty_params,

        // Set the initial EMA to 0
        ema: FPDecimal::zero(),

        // Know to fast forward to current net asset value if last_block == 0
        last_block: 0u64,
    };
    store_config(deps.storage, &cfg)?;
    Ok(Response::default())
}

/// ## Description
/// Compute EMA at the specific block height.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **block_height** is an object of type [`u64`] which is the height to compute EMA at.
///
/// - **net_asset_val** (NAV) is an object of type [`FPDecimal`] which is the sum of assets in
///     the inventory times their prices -- sum(asset_inv_i * price_i).
pub fn get_ema(deps: Deps, block_height: u64, net_asset_val: FPDecimal) -> StdResult<FPDecimal> {
    let cfg = read_config(deps.storage)?;
    // Get the previous rebalanced EMA
    let prev_ema = cfg.ema;
    // Get the previous rebalanced block
    let prev_block = cfg.last_block;
    if prev_block != 0u64 {
        // How many blocks has passed from the previous rebalance
        // -- dt = block_height - prev_block
        let dt = FPDecimal::from((block_height - prev_block) as u128);

        // Hard code one hour (600 blocks)
        // -- tau = -600
        let tau = FPDecimal::from(-600i128);
        // Weight ratio for EMA
        // -- factor = exp(dt/tau) = 1 / exp(dt/600)
        let factor = FPDecimal::_exp(dt / tau);

        // Compute EMA
        // -- EMA = factor * prev_ema + (1 - factor) * NAV
        Ok(factor * prev_ema + (FPDecimal::one() - factor) * net_asset_val)
    } else {
        // If this is the first rebalance, EMA is the current NAV
        Ok(net_asset_val)
    }
}

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             penalty_params,
///         }** Updates general penalty contract parameters.
///
/// - **ExecuteMsg::PenaltyCreate {
///             block_height,
///             cluster_token_supply,
///             inventory,
///             create_asset_amounts,
///             asset_prices,
///             target_weights,
///         }** Updates penalty contract states, EMA and last block, after a create operation.
///
/// - **ExecuteMsg::PenaltyRedeem {
///             block_height,
///             cluster_token_supply,
///             inventory,
///             max_tokens,
///             redeem_asset_amounts,
///             asset_prices,
///             target_weights,
///         }** Updates penalty contract states, EMA and last block, after a redeem operation.
///
/// ## Executor
/// Only the owner can execute this.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;

    // Permission check
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            penalty_params,
        } => update_config(deps, owner, penalty_params),
        ExecuteMsg::PenaltyCreate {
            block_height,
            cluster_token_supply,
            inventory,
            create_asset_amounts,
            asset_prices,
            target_weights,
        } => execute_mint(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &create_asset_amounts,
            &asset_prices,
            &target_weights,
        ),
        ExecuteMsg::PenaltyRedeem {
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

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **owner** is an object of type [`Option<String>`] which is the contract owner.
///
/// - **penalty_params** is an object of type [`Option<PenaltyParams>`] which are general
///     parameters for the penalty contract.
pub fn update_config(
    deps: DepsMut,
    owner: Option<String>,
    penalty_params: Option<PenaltyParams>,
) -> Result<Response, ContractError> {
    let api = deps.api;
    config_store(deps.storage).update(|mut config| -> StdResult<_> {
        if let Some(owner) = owner {
            // Validate address format
            config.owner = api.addr_validate(owner.as_str())?;
        }

        if let Some(penalty_params) = penalty_params {
            config.penalty_params = penalty_params;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Updates penalty contract states, EMA and last block, after a create operation.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **block_height** is an object of type [`u64`] is a specific height to compute mint at.
///
/// - [DEPRECATED] ~~**_cluster_token_supply** is a reference to an object of type [`Uint128`] which is the current
///     total supply for a cluster token.~~
///
/// - **inventory** is a reference to an array containing objects of type [`Uint128`] which is the
///     current inventory of inventory assets in a cluster.
///
/// - [DEPRECATED] ~~**_create_asset_amounts** is a reference to an array containing objects of type [`Uint128`] which
///     are the provided asset amounts for minting cluster tokens.~~
///
/// - **asset_prices** is a reference to an array containing objects of type [`String`] which are the
///     prices of the inventory assets in a cluster.
///
/// - [DEPRECATED] ~~**_target_weights** is a reference to an array containing objects of type [`Uint128`] which are
///     the current target weights of the assets in a cluster.~~
pub fn execute_mint(
    deps: DepsMut,
    block_height: u64,
    _cluster_token_supply: &Uint128,
    inventory: &[Uint128],
    _create_asset_amounts: &[Uint128],
    asset_prices: &[String],
    _target_weights: &[Uint128],
) -> Result<Response, ContractError> {
    // Retrieve the current asset inventory as `Vec<FPDecimal>`
    let i = int_vec_to_fpdec(inventory);
    // Retrieve the current asset prices as `Vec<FPDecimal>`
    let p = str_vec_to_fpdec(asset_prices)?;

    // Compute and update EMA and last block of the penalty contract
    update_ema(deps, block_height, dot(&i, &p))
}

/// ## Description
/// Updates penalty contract states, EMA and last block, after a create operation.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **block_height** is an object of type [`u64`] is a specific height to compute mint at.
///
/// - [DEPRECATED] ~~**_cluster_token_supply** is a reference to an object of type [`Uint128`] which is the current
///     total supply for a cluster token.~~
///
/// - **inventory** is a reference to an array containing objects of type [`Uint128`] which is the
///     current inventory of inventory assets in a cluster.
///
/// - [DEPRECATED] ~~**_max_tokens** is a reference to an object of type [`Uint128`] which is the required
///     maximum amount of cluster tokens allowed to burn for pro-rata redeem.~~
///
/// - [DEPRECATED] ~~**_redeem_asset_amounts** is a reference to an array containing objects of type [`Uint128`] which
///     are amounts expected to receive from burning cluster tokens.~~
///
/// - **asset_prices** is a reference to an array containing objects of type [`String`] which are the
///     prices of the inventory assets in a cluster.
///
/// - [DEPRECATED] ~~**_target_weights** is a reference to an array containing objects of type [`Uint128`] which are
///     the current target weights of the assets in a cluster.~~
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
) -> Result<Response, ContractError> {
    // Retrieve the current asset inventory as `Vec<FPDecimal>`
    let i = int_vec_to_fpdec(inventory);
    // Retrieve the current asset prices as `Vec<FPDecimal>`
    let p = str_vec_to_fpdec(asset_prices)?;

    // Compute and update EMA and last block of the penalty contract
    update_ema(deps, block_height, dot(&i, &p))
}

/// ## Description
/// Computes and updates the current EMA and last block in the penalty contract state.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **block_height** is an object of type [`u64`] which is a height to compute a new EMA.
///
/// - **net_asset_val** (NAV) is an object of type [`FPDecimal`] which is the sum of assets in
///     the inventory times their prices -- sum(asset_inv_i * price_i).
pub fn update_ema(
    deps: DepsMut,
    block_height: u64,
    net_asset_val: FPDecimal,
) -> Result<Response, ContractError> {
    let mut cfg = read_config(deps.storage)?;
    // Calculate and save the new EMA at the given `block_height`
    cfg.ema = get_ema(deps.as_ref(), block_height, net_asset_val)?;
    // Store `block_height` as the new last block
    cfg.last_block = block_height;

    // Save the state
    store_config(deps.storage, &cfg)?;
    Ok(Response::new().add_attributes(vec![attr("new_ema", cfg.ema.to_string())]))
}

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Params {}** Returns general contract parameters using a custom [`ParamsResponse`] structure.
///
/// - **QueryMsg::PenaltyQueryCreate {
///             block_height,
///             cluster_token_supply,
///             inventory,
///             create_asset_amounts,
///             asset_prices,
///             target_weights,
///         }** Calculates the actual create amount after taking penalty into consideration.
///
/// - **QueryMsg::PenaltyQueryRedeem {
///             block_height,
///             cluster_token_supply,
///             inventory,
///             max_tokens,
///             redeem_asset_amounts,
///             asset_prices,
///             target_weights,
///         }** Calculates the actual redeem amount after taking penalty into consideration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Params {} => to_binary(&get_params(deps)?),
        QueryMsg::PenaltyQueryCreate {
            block_height,
            cluster_token_supply,
            inventory,
            create_asset_amounts,
            asset_prices,
            target_weights,
        } => to_binary(&compute_mint(
            deps,
            block_height,
            &cluster_token_supply,
            &inventory,
            &create_asset_amounts,
            &asset_prices,
            &target_weights,
        )?),
        QueryMsg::PenaltyQueryRedeem {
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
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        penalty_params: state.penalty_params,
    };

    Ok(resp)
}

/// ## Description
/// Returns general contract parameters using a custom [`ParamsResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn get_params(deps: Deps) -> StdResult<ParamsResponse> {
    let cfg = read_config(deps.storage)?;
    Ok(ParamsResponse {
        penalty_params: cfg.penalty_params,
        last_block: cfg.last_block,
        ema: cfg.ema.to_string(),
    })
}

/// ## Description
/// Calculates the actual create amount after taking penalty into consideration.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **block_height** is an object of type [`u64`] is a specific height to compute mint at.
///
/// - **cluster_token_supply** is a reference to an object of type [`Uint128`] which is the current
///     total supply for a cluster token.
///
/// - **inventory** is a reference to an array containing objects of type [`Uint128`] which is the
///     current inventory of inventory assets in a cluster.
///
/// - **create_asset_amounts** is a reference to an array containing objects of type [`Uint128`] which
///     are the provided asset amounts for minting cluster tokens.
///
/// - **asset_prices** is a reference to an array containing objects of type [`String`] which are the
///     prices of the inventory assets in a cluster.
///
/// - **target_weights** is a reference to an array containing objects of type [`Uint128`] which are
///     the current target weights of the assets in a cluster.
pub fn compute_mint(
    deps: Deps,
    block_height: u64,
    cluster_token_supply: &Uint128,
    inventory: &[Uint128],
    create_asset_amounts: &[Uint128],
    asset_prices: &[String],
    target_weights: &[Uint128],
) -> StdResult<PenaltyCreateResponse> {
    // Current cluster token supply
    let n = FPDecimal::from(cluster_token_supply.u128());
    // The current inventory before adding the provided assets to mint
    let i0 = int_vec_to_fpdec(inventory);
    // The provided assets to mint
    let c = int_vec_to_fpdec(create_asset_amounts);
    // The current prices of the assets in the cluster
    let p = str_vec_to_fpdec(asset_prices)?;
    // The target weights of the assets
    let w = int_vec_to_fpdec(target_weights);

    // New inventory after adding the provided assets into the inventory
    let i1 = add(&i0, &c);

    // Compute penalty / reward from this rebalance
    // -- penalty if < 0
    // -- reward if > 0
    let penalty = notional_penalty(deps, block_height, &i0, &i1, &w, &p)?;
    // Compute the value of the provided assets with penalty
    // -- notional_value = value_of_the_provided_assets + penalty
    //                   = sum(provided_asset_i * price_i) + penalty
    let notional_value = dot(&c, &p) + penalty;

    // Compute the mint amount based on the ratio of the provided value and the total asset value (NAV)
    // -- mint = current_total_supply * (notional_value / net_asset_value)
    let mint_subtotal = n * notional_value / dot(&i0, &p);

    Ok(PenaltyCreateResponse {
        create_tokens: Uint128::new(mint_subtotal.into()),
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

/// ## Description
/// Calculates the actual redeem amount after taking penalty into consideration.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **block_height** is an object of type [`u64`] is a specific height to compute mint at.
///
/// - **cluster_token_supply** is a reference to an object of type [`Uint128`] which is the current
///     total supply for a cluster token.
///
/// - **inventory** is a reference to an array containing objects of type [`Uint128`] which is the
///     current inventory of inventory assets in a cluster.
///
/// - **max_tokens** is a reference to an object of type [`Uint128`] which is the required
///     maximum amount of cluster tokens allowed to burn for pro-rata redeem.
///
/// - **redeem_asset_amounts** is a reference to an array containing objects of type [`Uint128`] which
///     are amounts expected to receive from burning cluster tokens.
///
/// - **asset_prices** is a reference to an array containing objects of type [`String`] which are the
///     prices of the inventory assets in a cluster.
///
/// - **target_weights** is a reference to an array containing objects of type [`Uint128`] which are
///     the current target weights of the assets in a cluster.
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
) -> StdResult<PenaltyRedeemResponse> {
    // Current cluster token supply
    let n = FPDecimal::from(cluster_token_supply.u128());
    // The current inventory before adding the provided assets to mint
    let i0 = int_vec_to_fpdec(inventory);
    // Max cluster token amount allowed to burn
    let m = FPDecimal::from(max_tokens.u128());
    // The expected return assets
    let r = int_vec_to_fpdec(redeem_asset_amounts);
    // The current prices of the assets in the cluster
    let p = str_vec_to_fpdec(asset_prices)?;
    // The target weights of the assets
    let w = int_vec_to_fpdec(target_weights);

    return if redeem_asset_amounts.is_empty() {
        // No expected return assets, use pro-rata redeem

        // Compute pro-rata redeem based on the current inventory
        // No need to compute penalty since pro-rate does not move the inventory ratio
        // -- redeem_arr = current_inventory * (max_tokens / current_total_supply)
        let redeem_arr = div_const(&mul_const(&i0, m), n);
        Ok(PenaltyRedeemResponse {
            token_cost: Uint128::new(m.into()),
            penalty: Uint128::zero(),
            redeem_assets: redeem_arr
                .iter()
                .map(|&x| Uint128::new(x.into()))
                .collect::<Vec<Uint128>>(),
            attributes: vec![],
        })
    } else {
        // New inventory after removing the assets expected from burning
        let i1 = sub(&i0, &r);

        // Compute penalty / reward from this rebalance
        // -- penalty if < 0
        // -- reward if > 0
        let penalty = notional_penalty(deps, block_height, &i0, &i1, &w, &p)?;
        // Compute the value of the returned assets with penalty
        // -- notional_value = value_of_the_returned_assets - penalty
        //                   = sum(provided_asset_i * price_i) - penalty
        let notional_value = dot(&r, &p) - penalty;

        // Compute the actual tokens needed based on the ratio of the returned value and the total asset value (NAV)
        // -- burn = current_total_supply * (notional_value / net_asset_value)
        let needed_tokens = n * notional_value / dot(&i0, &p);

        // Ceil up the amount of token cost
        let mut token_cost = needed_tokens.into();
        if needed_tokens != FPDecimal::from(token_cost) {
            token_cost += 1;
        }

        Ok(PenaltyRedeemResponse {
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

/// ## Description
/// Calculates penalty / reward for any rebalance operation on a cluster.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **block_height** is an object of type [`u64`].
///
/// - **i0** is a reference to an array containing objects of type [`FPDecimal`] which is
///     the current inventory of a cluster.
///
/// - **i1** is a reference to an array containing objects of type [`FPDecimal`] which is
///     the inventory of a cluster after the rebalance operation.
///
/// - **w** is a reference to an array containing objects of type [`FPDecimal`] which is
///     a list of asset target weights of a cluster.
///
/// - **p** is a reference to an array containing objects of type [`FPDecimal`] which is
///     a list of asset prices of a cluster.
pub fn notional_penalty(
    deps: Deps,
    block_height: u64,
    i0: &[FPDecimal],
    i1: &[FPDecimal],
    w: &[FPDecimal],
    p: &[FPDecimal],
) -> StdResult<FPDecimal> {
    let cfg = read_config(deps.storage)?;

    // Compute the current imbalance with `i0`
    let imb0 = imbalance(i0, p, w);
    // Compute the imbalance after the rebalance with `i1`
    let imb1 = imbalance(i1, p, w);

    // e is the minimum of the EMA and the net asset value
    // -- It is important to not let e exceed NAV to prevent someone
    //    pumping e to "stretch" penalty_cutoff_hi and then using it to
    //    duck the cluster imbalance too high issue
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
        // Imbalance increases, use penalty function
        let cutoff_lo = penalty_cutoff_lo * e;
        let cutoff_hi = penalty_cutoff_hi * e;

        if imb1 > cutoff_hi {
            return Err(StdError::generic_err("cluster imbalance too high"));
        }

        // Penalty function is broken into three pieces, where its flat, linear, and then flat
        // Compute the area under each piece separately

        let penalty_1 = (min(imb1, cutoff_lo) - min(imb0, cutoff_lo)) * penalty_amt_lo;

        // Clip to only middle portion
        let imb0_mid = min(max(imb0, cutoff_lo), cutoff_hi);
        let imb1_mid = min(max(imb1, cutoff_lo), cutoff_hi);

        let amt_gap = penalty_amt_hi - penalty_amt_lo;
        let cutoff_gap = cutoff_hi - cutoff_lo;

        // Value of y when x is at imb0_mid and imb1_mid respectively
        let imb0_mid_height = (imb0_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;
        let imb1_mid_height = (imb1_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo;

        // Area of a trapezoid
        let penalty_2 = (imb0_mid_height + imb1_mid_height) * (imb1_mid - imb0_mid).div(2);

        let penalty_3 = (max(imb1, cutoff_hi) - max(imb0, cutoff_hi)) * penalty_amt_hi;
        Ok(FPDecimal::zero() - (penalty_1 + penalty_2 + penalty_3))
    } else {
        // Imbalance decreases, use reward function
        let cutoff = reward_cutoff * e;
        Ok((max(imb0, cutoff) - max(imb1, cutoff)) * reward_amt)
    }
}

/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
