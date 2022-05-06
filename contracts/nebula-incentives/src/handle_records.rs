use crate::query::query_config;
use crate::state::record_contribution;
use cosmwasm_std::{
    attr, to_binary, Addr, DepsMut, Env, MessageInfo, QueryRequest, Response, Uint128, WasmQuery,
};

use nebula_protocol::incentives::PoolType;

use astroport::asset::{Asset, AssetInfo};
use astroport::pair::PoolResponse as AstroportPoolResponse;
use astroport::pair::QueryMsg as AstroportQueryMsg;

use nebula_protocol::cluster::{ClusterStateResponse, QueryMsg as ClusterQueryMsg};
use nebula_protocol::incentives::ConfigResponse;
use nebula_protocol::penalty::{PenaltyNotionalResponse, QueryMsg as PenaltyQueryMsg};

use crate::error::ContractError;
use cluster_math::FPDecimal;
use std::str::FromStr;

/// ## Description
/// Saves the change occurs in the Astroport pair pool after performing an arbitrage action.
/// This is used to calculate contribution rewards when arbitraging.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **arbitrageur** is an object of type [`Addr`] which is the address of a user
///     performing an arbitrage.
///
/// - **astroport_pair** is an object of type [`Addr`] which is the address of
///     the Astroport pair contract that the arbitrage is executed on.
///
/// - **cluster_contract** is an object of type [`Addr`] which is the address of
///     the cluster contract corresponding to the arbitrage.
///
/// - **pool_before** is an object of type [`AstroportPoolResponse`] which is the state
///     of the Astroport pair pool before performing the arbitrage.
///
/// ## Executor
/// Only this contract can execute this.
pub fn record_astroport_impact(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    arbitrageur: Addr,
    astroport_pair: Addr,
    cluster_contract: Addr,
    pool_before: AstroportPoolResponse,
) -> Result<Response, ContractError> {
    let config: ConfigResponse = query_config(deps.as_ref())?;
    // Permission check
    if info.sender != config.proxy {
        return Err(ContractError::Unauthorized {});
    }

    // Get the current state of the Astroport pair pool
    let pool_now: AstroportPoolResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: astroport_pair.to_string(),
            msg: to_binary(&AstroportQueryMsg::Pool {})?,
        }))?;

    // Get the state of the cluster
    let contract_state: ClusterStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cluster_contract.to_string(),
            msg: to_binary(&ClusterQueryMsg::ClusterState {})?,
        }))?;

    // Compute Net Asset Value (NAV) of the cluster
    // -- fair_value (NAV) = sum(price_i * inv_i) / CT_total_supply
    let mut fair_value = FPDecimal::zero();
    for i in 0..contract_state.prices.len() {
        fair_value = fair_value
            + FPDecimal::from_str(&*contract_state.prices[i])?
                * FPDecimal::from(contract_state.inv[i].u128());
    }
    fair_value = fair_value / FPDecimal::from(contract_state.outstanding_balance_tokens.u128());

    // Unfortunately the product increases with the transaction (due to Astroport fee)
    // which causes cases where the prices moves in the right direction
    // but the imbalance computed here goes up
    // hopefully they are rare enough to ignore
    fn astroport_imbalance(assets: &[Asset], fair_value: FPDecimal) -> FPDecimal {
        let sorted_assets = match assets[0].clone().info {
            AssetInfo::Token { .. } => vec![assets[1].clone(), assets[0].clone()],
            AssetInfo::NativeToken { .. } => assets.to_vec(),
        };

        // UST amount in the Astroport pool
        let amt_denom = FPDecimal::from(sorted_assets[0].amount.u128());
        // Cluster token (CT) amount in the Astroport pool
        let amt_ct = FPDecimal::from(sorted_assets[1].amount.u128());

        // Compute the current k = xy = UST_amount * CT_amount
        let k = amt_denom * amt_ct;

        // How much dollars needs to move to set this cluster back into balance?
        // First compute what the pool should look like if optimally balanced
        // `true_amt_denom` and `true_amt_ct` represent what the pool should look like
        // -- true_amt_denom = true_amt_ct * fair_value    __(1)
        // -- true_amt_ct = prod / true_amt_denom          __(2)
        // (1) + (2),
        // -- true_amt_denom = prod / true_amt_denom * fair_value
        // -- true_amt_denom = sqrt(prod * fair_value)
        let true_amt_denom = FPDecimal::_pow(k * fair_value, FPDecimal::one().div(2i128));
        (amt_denom - true_amt_denom).abs()
    }

    // Calculate the Astrport pool imbalance before the arbitrage
    let imb0 = astroport_imbalance(&pool_before.assets.to_vec(), fair_value);
    // Calculate the Astrport pool imbalance after the arbitrage
    let imb1 = astroport_imbalance(&pool_now.assets.to_vec(), fair_value);

    // If positive, this arbitrage moved the market price closer to fair value (NAV)
    let imbalance_fixed = imb0 - imb1;
    if imbalance_fixed.sign == 1 {
        let imbalanced_fixed = Uint128::new(imbalance_fixed.into());
        record_contribution(
            deps,
            &arbitrageur,
            PoolType::ARBITRAGE,
            &cluster_contract,
            Uint128::new(imbalanced_fixed.into()),
        )?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "record_astroport_arbitrageur_rewards"),
        attr("fair_value", &format!("{}", fair_value)),
        attr("arbitrage_imbalance_fixed", &format!("{}", imbalance_fixed)),
        attr("arbitrage_imbalance_sign", imbalance_fixed.sign.to_string()),
        attr("imb0", &format!("{}", imb0)),
        attr("imb1", &format!("{}", imb1)),
    ]))
}

/// ## Description
/// Saves the change occurs in the cluster inventory after performing a rebalance action.
/// This is used to calculate contribution rewards when rebalancing.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **rebalancer** is an object of type [`Addr`] which is the address of a user
///     performing a rebalance.
///
/// - **cluster_contract** is an object of type [`Addr`] which is the address of
///     the cluster contract corresponding to the rebalance.
///
/// - **original_imbalance** is an object of type [`Uint128`] which is the imbalance
///     value of the cluster before performing the rebalance.
///
/// ## Executor
/// Only this contract can execute this.
pub fn record_rebalancer_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    original_inventory: Vec<Uint128>,
) -> Result<Response, ContractError> {
    let config: ConfigResponse = query_config(deps.as_ref())?;
    // Permission check
    if info.sender != config.proxy {
        return Err(ContractError::Unauthorized {});
    }

    // Query the cluster state
    let cluster_state: ClusterStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cluster_contract.to_string(),
            msg: to_binary(&ClusterQueryMsg::ClusterState {})?,
        }))?;
    // Get the asset target weights of the cluster
    let target_weights = cluster_state
        .target
        .iter()
        .map(|x| x.amount)
        .collect::<Vec<_>>();

    // Get the penalty and both imbalances before and after rebalance
    let penalty_response: PenaltyNotionalResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cluster_state.penalty.to_string(),
            msg: to_binary(&PenaltyQueryMsg::PenaltyQueryNotional {
                block_height: env.block.height,
                inventory0: original_inventory,
                inventory1: cluster_state.inv,
                asset_prices: cluster_state.prices,
                target_weights,
            })?,
        }))?;

    let mut contribution = Uint128::zero();

    // If imbalance reduces
    if penalty_response.penalty > Uint128::zero() {
        contribution = penalty_response
            .imbalance0
            .checked_sub(penalty_response.imbalance1)?;

        // Save the rebalance contribution
        record_contribution(
            deps,
            &rebalancer,
            PoolType::REBALANCE,
            &cluster_contract,
            contribution,
        )?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "record_rebalancer_rewards"),
        attr("rebalancer_imbalance_fixed", contribution),
    ]))
}
