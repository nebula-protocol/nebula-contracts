use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::rebalancers::{assert_cluster_exists, get_cluster_state};
use crate::state::{read_config, record_contribution, Config};

use nebula_protocol::incentives::{ExecuteMsg, PoolType};

use astroport::pair::PoolResponse as AstroportPoolResponse;
use astroport::pair::QueryMsg as AstroportQueryMsg;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::{Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg};
use astroport::querier::{query_balance, query_pair_info, query_token_balance};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{ClusterStateResponse, QueryMsg as ClusterQueryMsg};

use cluster_math::FPDecimal;
use std::str::FromStr;

pub fn get_pair_info(deps: Deps, cluster_token: &String) -> StdResult<PairInfo> {
    let config: Config = read_config(deps.storage)?;
    let astroport_factory_raw = config.astroport_factory;
    query_pair_info(
        &deps.querier,
        Addr::unchecked(astroport_factory_raw),
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom,
            },
            AssetInfo::Token {
                contract_addr: deps.api.addr_validate(cluster_token.as_str())?,
            },
        ],
    )
}

// UST -> Assets
// 1. swap_all
// 2. record difference
// 3. redeem
// 4. send_all
// pub fn ust_to_asset_tokens(
//     deps: DepsMut,
//     env: Env,
//     cluster_contract: &String,
//     assets: &Vec<Asset>,
// ) -> StdResult<Response> {
//
// }
// Assets -> UST
// 1. mint
// 2. swap_all
// 3. record difference
// 4. send_all

pub fn arb_cluster_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    assets: &[Asset],
    min_ust: Option<Uint128>,
) -> StdResult<Response> {
    assert_cluster_exists(deps.as_ref(), &cluster_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address;

    let cfg: Config = read_config(deps.storage)?;

    let cluster_state = get_cluster_state(deps.as_ref(), &cluster_contract)?;

    let cluster_token = cluster_state.cluster_token;

    let pair_info = get_pair_info(deps.as_ref(), &cluster_token)?;

    // transfer all asset tokens into this
    // also prepare to transfer to cluster contract
    for asset in assets {
        match asset.clone().info {
            AssetInfo::NativeToken { denom: _ } => {
                asset.clone().assert_sent_native_token_balance(&info)?
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: contract.to_string(),
                        amount: asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedCreate {
            rebalancer: info.sender.to_string(),
            cluster_contract: cluster_contract.clone(),
            asset_amounts: assets.to_vec(),
            min_tokens: None,
        })?,
        funds: vec![],
    }));

    // swap all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SwapAll {
            astroport_pair: pair_info.contract_addr.to_string(),
            cluster_token,
            to_ust: true, // how about changing this to to_base
            min_return: min_ust.unwrap_or(Uint128::zero()),
        })?,
        funds: vec![],
    }));

    // record pool state difference
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
            arbitrageur: info.sender.to_string(),
            astroport_pair: pair_info.contract_addr.to_string(),
            cluster_contract,
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_binary(&AstroportQueryMsg::Pool {})?,
            }))?,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos: vec![AssetInfo::NativeToken {
                denom: cfg.base_denom,
            }],
            send_to: info.sender.to_string(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "arb_cluster_create"),
        attr("sender", info.sender.as_str()),
    ]))
}

pub fn arb_cluster_redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    asset: Asset,
    min_cluster: Option<Uint128>,
) -> StdResult<Response> {
    assert_cluster_exists(deps.as_ref(), &cluster_contract)?;

    let cluster_state = get_cluster_state(deps.as_ref(), &cluster_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address;

    let cfg: Config = read_config(deps.storage)?;

    match asset.info {
        AssetInfo::Token { .. } => return Err(StdError::generic_err("not native token")),
        AssetInfo::NativeToken { ref denom } => {
            if denom.clone() != cfg.base_denom {
                return Err(StdError::generic_err("wrong base denom"));
            }
        }
    };

    asset.assert_sent_native_token_balance(&info)?;

    let cluster_token = cluster_state.cluster_token;

    let pair_info = get_pair_info(deps.as_ref(), &cluster_token)?;

    // swap all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SwapAll {
            astroport_pair: pair_info.contract_addr.to_string(),
            cluster_token: cluster_token.clone(),
            to_ust: false,
            min_return: min_cluster.unwrap_or(Uint128::zero()),
        })?,
        funds: vec![],
    }));

    // record pool state difference
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
            arbitrageur: info.sender.to_string(),
            astroport_pair: pair_info.contract_addr.to_string(),
            cluster_contract: cluster_contract.clone(),
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_binary(&AstroportQueryMsg::Pool {})?,
            }))?,
        })?,
        funds: vec![],
    }));

    // redeem cluster token
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
            rebalancer: info.sender.to_string(),
            cluster_contract,
            cluster_token: cluster_token,
            max_tokens: None,
            asset_amounts: None,
        })?,
        funds: vec![],
    }));

    let asset_infos = cluster_state
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    // send all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos,
            send_to: info.sender.to_string(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "arb_cluster_redeem"),
        attr("sender", info.sender.as_str()),
    ]))
}

pub fn record_astroport_impact(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    arbitrageur: String,
    astroport_pair: String,
    cluster_contract: String,
    pool_before: AstroportPoolResponse,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let pool_now: AstroportPoolResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: astroport_pair,
            msg: to_binary(&AstroportQueryMsg::Pool {})?,
        }))?;

    let contract_state: ClusterStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cluster_contract.clone(),
            msg: to_binary(&ClusterQueryMsg::ClusterState {})?,
        }))?;

    // here we compute the "fair" value of a cluster token
    // by breaking it down into its respective components
    // is that the real fair value? to actually extract
    // this value you need to pay significant fees,
    // so realistically the fair value on astroport
    // may be 0-2% cheaper
    let mut fair_value = FPDecimal::zero();
    for i in 0..contract_state.prices.len() {
        fair_value = fair_value
            + FPDecimal::from_str(&*contract_state.prices[i])?
                * FPDecimal::from(contract_state.inv[i].u128());
    }

    fair_value = fair_value / FPDecimal::from(contract_state.outstanding_balance_tokens.u128());

    // unfortunately the product increases with the transaction
    // which causes cases where the prices moves in the right direction
    // but the imbalance computed here goes up
    // hopefully they are rare enough to ignore
    fn astroport_imbalance(assets: &[Asset], fair_value: FPDecimal) -> FPDecimal {
        let sorted_assets = match assets[0].clone().info {
            AssetInfo::Token { .. } => vec![assets[1].clone(), assets[0].clone()],
            AssetInfo::NativeToken { .. } => assets.to_vec(),
        };

        let amt_denom = FPDecimal::from(sorted_assets[0].amount.u128());
        let amt_bsk = FPDecimal::from(sorted_assets[1].amount.u128());
        let prod = amt_denom * amt_bsk;

        // how much dollars needs to move to set this cluster back into balance?
        // first compute what the pool should look like if optimally balanced
        // true_denom, true_bsk represent what the pool should look like
        // true_denom = true_bsk * fair_value
        // true_bsk = prod / true_denom
        // true_denom = prod / true_denom * fair_value
        // true_denom = sqrt(prod * fair_value)

        let true_denom = FPDecimal::_pow(prod * fair_value, FPDecimal::one().div(2i128));
        (amt_denom - true_denom).abs()
    }

    // if positive -> this arb moved us closer to fair value
    let imb0 = astroport_imbalance(&pool_before.assets.to_vec(), fair_value);
    let imb1 = astroport_imbalance(&pool_now.assets.to_vec(), fair_value);

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
// either UST -> BSK or BSK -> UST, swap all inventory
// we can do this because this contract never holds any inventory
// between transactions
pub fn swap_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    astroport_pair: String,
    cluster_token: String,
    to_ust: bool,
    min_return: Uint128,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let config: Config = read_config(deps.storage)?;
    let mut messages = vec![];

    let mut logs = vec![
        attr("action", "swap_all"),
        attr("to_usd", to_ust.to_string()),
    ];

    if to_ust {
        let amount = query_token_balance(
            &deps.querier,
            Addr::unchecked(cluster_token.to_string()),
            env.contract.address,
        )?;
        let belief_price = if min_return == Uint128::zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(amount, min_return)
        };

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: astroport_pair.clone(),
                amount,
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: Some(Decimal::zero()),
                    belief_price: Some(belief_price),
                    to: None,
                })?,
            })?,
            funds: vec![],
        }));
        logs.push(attr("amount", amount));
        logs.push(attr("addr", astroport_pair));
    } else {
        let amount = query_balance(
            &deps.querier,
            env.contract.address,
            config.base_denom.to_string(),
        )?;

        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // deduct tax first
        let amount = (swap_asset.deduct_tax(&deps.querier)?).amount;
        let belief_price = if min_return == Uint128::zero() {
            Decimal::zero()
        } else {
            Decimal::from_ratio(amount, min_return)
        };

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: astroport_pair,
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    amount,
                    ..swap_asset
                },
                max_spread: Some(Decimal::zero()),
                belief_price: Some(belief_price),
                to: None,
            })?,
            funds: vec![Coin {
                denom: config.base_denom,
                amount,
            }],
        }));
    }
    Ok(Response::new().add_messages(messages).add_attributes(logs))
}

pub fn send_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: &[AssetInfo],
    send_to: String,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let mut messages = vec![];

    for asset_info in asset_infos {
        let asset = Asset {
            info: asset_info.clone(),
            amount: match asset_info {
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    Addr::unchecked(contract_addr.to_string()),
                    env.contract.address.clone(),
                )?,
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?
                }
            },
        };
        if asset.amount > Uint128::zero() {
            messages.push(asset.into_msg(&deps.querier, Addr::unchecked(send_to.clone()))?);
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "send_all")]))
}
