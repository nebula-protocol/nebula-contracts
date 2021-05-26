use cosmwasm_std::{
    log, to_binary, Api, Coin, CosmosMsg, Env, Extern, HandleResponse,
    HumanAddr, Querier, QueryRequest,
    StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{read_config, Config, record_contribution};
use crate::rebalancers::{assert_cluster_exists};

use nebula_protocol::incentives::{ExtQueryMsg, HandleMsg, PoolResponse, PoolType};

use cw20::{Cw20HandleMsg};
use nebula_protocol::cluster::{
    BasketStateResponse, ConfigResponse as ClusterConfigResponse,
    QueryMsg as ClusterQueryMsg,
};
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, HandleMsg as TerraswapHandleMsg};
use terraswap::querier::{query_balance, query_pair_info, query_token_balance};

use basket_math::FPDecimal;
use std::str::FromStr;

pub fn get_pair_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_token: &HumanAddr,
) -> StdResult<PairInfo> {
    let config: Config = read_config(&deps.storage)?;
    let terraswap_factory_raw = deps.api.human_address(&config.terraswap_factory)?;
    return query_pair_info(
        &deps,
        &terraswap_factory_raw,
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom.to_string(),
            },
            AssetInfo::Token {
                contract_addr: basket_token.clone(),
            },
        ],
    );
}

// UST -> Assets
// 1. swap_all
// 2. record difference
// 3. redeem
// 4. send_all
// pub fn ust_to_asset_tokens<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     basket_contract: &HumanAddr,
//     assets: &Vec<Asset>,
// ) -> StdResult<HandleResponse> {
//
// }
// Assets -> UST
// 1. mint
// 2. swap_all
// 3. record difference
// 4. send_all

pub fn arb_cluster_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_contract: &HumanAddr,
    assets: &Vec<Asset>,
) -> StdResult<HandleResponse> {

    assert_cluster_exists(deps, &basket_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address.clone();

    let cfg: Config = read_config(&deps.storage)?;

    let basket_config_response: ClusterConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: basket_contract.clone(),
            msg: to_binary(&ClusterQueryMsg::Config {})?,
        }))?;

    let basket_config = basket_config_response.config;
    let basket_token = basket_config.basket_token.unwrap();

    let pair_info = get_pair_info(deps, &basket_token)?;

    let mut send = vec![];

    // transfer all asset tokens into this
    // also prepare to transfer to basket contract
    for asset in assets {
        match asset.clone().info {
            AssetInfo::NativeToken { denom } => {
                asset.clone().assert_sent_native_token_balance(&env)?;
                send.push(Coin {
                    denom,
                    amount: asset.amount,
                })
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&Cw20HandleMsg::TransferFrom {
                        owner: env.message.sender.clone(),
                        recipient: contract.clone(),
                        amount: asset.amount,
                    })?,
                    send: vec![],
                }));


            }
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::_InternalRewardedMint {
            rebalancer: env.message.sender.clone(),
            basket_contract: basket_contract.clone(),
            asset_amounts: assets.clone(),
            min_tokens: None,
        })?,
        send,
    }));

    // swap all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::SwapAll {
            terraswap_pair: pair_info.contract_addr.clone(),
            basket_token: basket_token.clone(),
            to_ust: true,
        })?,
        send: vec![],
    }));

    // record pool state difference
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::RecordTerraswapImpact {
            arbitrager: env.message.sender.clone(),
            terraswap_pair: pair_info.contract_addr.clone(),
            basket_contract: basket_contract.clone(),
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.clone(),
                msg: to_binary(&ExtQueryMsg::Pool {})?,
            }))?,
        })?,
        send: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::SendAll {
            asset_infos: vec![AssetInfo::NativeToken {
                denom: cfg.base_denom.clone(),
            }],
            send_to: env.message.sender,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}

pub fn arb_cluster_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_contract: &HumanAddr,
    asset: &Asset,
) -> StdResult<HandleResponse> {

    assert_cluster_exists(deps, &basket_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address.clone();

    let cfg: Config = read_config(&deps.storage)?;


    let mut swap_coins = vec![];

    match asset.info {
        AssetInfo::Token { .. } => return Err(StdError::generic_err("not native token")),
        AssetInfo::NativeToken { ref denom } => {
            if denom.clone() != cfg.base_denom {
                return Err(StdError::generic_err("wrong base denom"));
            }
            swap_coins.push(
                Coin {
                    denom: denom.clone(),
                    amount: asset.amount,
                }
            )
        }
    };

    asset.assert_sent_native_token_balance(&env)?;

    let basket_config_response: ClusterConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: basket_contract.clone(),
            msg: to_binary(&ClusterQueryMsg::Config {})?,
        }))?;

    let basket_config = basket_config_response.config;
    let basket_token = basket_config.basket_token.unwrap();

    let pair_info = get_pair_info(deps, &basket_token)?;

    // swap all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::SwapAll {
            terraswap_pair: pair_info.contract_addr.clone(),
            basket_token: basket_token.clone(),
            to_ust: false,
        })?,
        send: swap_coins
    }));

    // record pool state difference
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::RecordTerraswapImpact {
            arbitrager: env.message.sender.clone(),
            terraswap_pair: pair_info.contract_addr.clone(),
            basket_contract: basket_contract.clone(),
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.clone(),
                msg: to_binary(&ExtQueryMsg::Pool {})?,
            }))?,
        })?,
        send: vec![],
    }));

    // redeem basket token
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::_InternalRewardedRedeem {
            rebalancer: env.message.sender.clone(),
            basket_contract: basket_contract.clone(),
            basket_token: basket_token.clone(),
            max_tokens: None,
            asset_amounts: None,
        })?,
        send: vec![],
    }));

    // send all
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::SendAll {
            asset_infos: vec![AssetInfo::Token { contract_addr: basket_token.clone() }],
            send_to: env.message.sender,
        })?,
        send: vec![],
    }));
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}

pub fn record_terraswap_impact<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    arbitrager: &HumanAddr,
    terraswap_pair: &HumanAddr,
    basket_contract: &HumanAddr,
    pool_before: &PoolResponse,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let pool_now: PoolResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: terraswap_pair.clone(),
        msg: to_binary(&ExtQueryMsg::Pool {})?,
    }))?;

    let arbitrager = deps.api.canonical_address(&arbitrager)?;

    let contract_state: BasketStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: basket_contract.clone(),
            msg: to_binary(&ClusterQueryMsg::BasketState {
                basket_contract_address: basket_contract.clone(),
            })?,
        }))?;

    // here we compute the "fair" value of a basket token
    // by breaking it down into its respective components
    // is that the real fair value? to actually extract
    // this value you need to pay significant fees,
    // so realistically the fair value on terraswap
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
    fn terraswap_imbalance(assets: &Vec<Asset>, fair_value: FPDecimal) -> FPDecimal {
        let sorted_assets = match assets[0].clone().info {
            AssetInfo::Token { .. } => vec![assets[1].clone(), assets[0].clone()],
            AssetInfo::NativeToken { .. } => assets.to_vec(),
        };

        let amt_denom = FPDecimal::from(sorted_assets[0].amount.u128());
        let amt_bsk = FPDecimal::from(sorted_assets[1].amount.u128());
        let prod = amt_denom * amt_bsk;

        // how much dollars needs to move to set this basket back into balance?
        // first compute what the pool should look like if optimally balanced
        // true_denom = true_bsk * fair_value
        // true_bsk = prod / true_denom
        // true_denom = prod / true_denom * fair_value
        // true_denom = sqrt(prod * fair_value)

        let true_denom = FPDecimal::_pow(prod * fair_value, FPDecimal::one().div(2i128));
        return (amt_denom - true_denom).abs();
    }
    // if positive -> this arb moved us closer to fair value
    let imb0 = terraswap_imbalance(&pool_before.assets.to_vec(), fair_value);
    let imb1 = terraswap_imbalance( &pool_now.assets.to_vec(), fair_value);

    let imbalance_fixed = imb0 - imb1;

    if imbalance_fixed.sign == 1 {
        let imbalanced_fixed = Uint128(imbalance_fixed.into());
        record_contribution(
            deps,
            &arbitrager,
            PoolType::ARBITRAGER,
            &deps.api.canonical_address(basket_contract)?,
            Uint128(imbalanced_fixed.into()),
        )?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("fair_value", fair_value),
            log("arbitrage_imbalance_fixed", imbalance_fixed),
            log("arbitrage_imbalance_sign", imbalance_fixed.sign),
            log("imb0", imb0),
            log("imb1", imb1),
        ],
        data: None,
    })
}
// either UST -> BSK or BSK -> UST, swap all inventory
// we can do this because this contract never holds any inventory
// between transactions
pub fn swap_all<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    terraswap_pair: &HumanAddr,
    basket_token: &HumanAddr,
    to_ust: bool,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let config: Config = read_config(&deps.storage)?;
    let mut messages = vec![];

    let mut logs = vec![log("action", "swap_all"), log("to_usd", to_ust)];

    if to_ust {
        let amount = query_token_balance(&deps, &basket_token, &env.contract.address)?;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: basket_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: terraswap_pair.clone(),
                amount,
                msg: Some(to_binary(&TerraswapCw20HookMsg::Swap {
                    max_spread: None,
                    belief_price: None,
                    to: None,
                })?),
            })?,
            send: vec![],
        }));
        logs.push(log("amount", amount));
        logs.push(log("addr", terraswap_pair.clone().to_string()));
    } else {
        let amount = query_balance(&deps, &env.contract.address, config.base_denom.to_string())?;
        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // deduct tax first
        let amount = (swap_asset.deduct_tax(&deps)?).amount;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: terraswap_pair.clone(),
            msg: to_binary(&TerraswapHandleMsg::Swap {
                offer_asset: Asset {
                    amount,
                    ..swap_asset
                },
                max_spread: None,
                belief_price: None,
                to: None,
            })?,
            send: vec![Coin {
                denom: config.base_denom,
                amount,
            }],
        }));
    }
    Ok(HandleResponse {
        messages,
        log: logs,
        data: None,
    })
}

// make sure this is airtight so the someone cannot send himself all of the nebula in this contract
pub fn send_all<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_infos: &Vec<AssetInfo>,
    send_to: &HumanAddr,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let mut messages = vec![];

    for asset_info in asset_infos {
        let asset = Asset {
            info: asset_info.clone(),
            amount: match asset_info {
                AssetInfo::Token { contract_addr } => {
                    query_token_balance(&deps, &contract_addr, &env.contract.address)?
                }
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps, &env.contract.address, denom.clone())?
                }
            },
        };
        if asset.amount > Uint128::zero() {
            messages.push(asset.into_msg(&deps, env.contract.address.clone(), send_to.clone())?);
        }
    }

    Ok(HandleResponse {
        messages,
        log: vec![log("action", "send_all")],
        data: None,
    })
}