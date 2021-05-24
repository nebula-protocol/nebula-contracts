use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, InitResponse, MigrateResponse, MigrateResult, Querier, QueryRequest,
    StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};

use crate::rewards::{deposit_reward, increment_n, record_penalty, withdraw_reward};
use crate::state::{read_config, store_config, store_current_n, Config};
use nebula_protocol::gov::Cw20HookMsg as GovCw20HookMsg;
use nebula_protocol::incentives::{
    ConfigResponse, Cw20HookMsg, ExtQueryMsg, HandleMsg, InitMsg, MigrateMsg, PoolResponse,
    QueryMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use nebula_protocol::cluster::QueryMsg::BasketState;
use nebula_protocol::cluster::{
    BasketConfig, BasketStateResponse, ConfigResponse as ClusterConfigResponse,
    HandleMsg as ClusterHandleMsg, QueryMsg as ClusterQueryMsg,
};
use nebula_protocol::factory::ClusterExistsResponse;
use nebula_protocol::factory::QueryMsg::ClusterExists;
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, HandleMsg as TerraswapHandleMsg};
use terraswap::querier::{query_balance, query_pair_info, query_token_balance};

use basket_math::FPDecimal;
use std::str::FromStr;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            factory: deps.api.canonical_address(&msg.factory)?,
            terraswap_factory: deps.api.canonical_address(&msg.terraswap_factory)?,
            nebula_token: deps.api.canonical_address(&msg.nebula_token)?,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    store_current_n(&mut deps.storage, 0)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, env, &owner),
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::RecordPenalty {
            asset_address,
            reward_owner,
            penalty_amount,
        } => record_penalty(deps, env, &reward_owner, &asset_address, penalty_amount),
        HandleMsg::Withdraw {} => withdraw_reward(deps, env),
        HandleMsg::NewPenaltyPeriod {} => new_penalty_period(deps, env),
        HandleMsg::SwapAll {
            terraswap_pair,
            basket_token,
            to_ust,
        } => swap_all(deps, env, &terraswap_pair, &basket_token, to_ust),
        HandleMsg::SendAll {
            asset_infos,
            send_to,
        } => send_all(deps, env, &asset_infos, &send_to),
        HandleMsg::RecordTerraswapImpact {
            terraswap_pair,
            basket_contract,
            pool_before,
        } => record_terraswap_impact(deps, env, &terraswap_pair, &basket_contract, &pool_before),
        HandleMsg::RedeemAll {
            basket_contract,
            basket_token,
        } => redeem_all(deps, env, &basket_contract, &basket_token),
        HandleMsg::ArbClusterMint {
            basket_contract,
            assets,
        } => arb_cluster_mint(deps, env, &basket_contract, &assets),
        HandleMsg::ArbClusterRedeem {
            basket_contract,
            asset,
        } => arb_cluster_redeem(deps, env, &basket_contract, &asset),
    }
}

pub fn try_reset_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let mut new_cfg = cfg.clone();
    new_cfg.owner = owner.clone();
    store_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "_try_reset_owner")],
        data: None,
    })
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    if let Some(msg) = cw20_msg.msg {
        let config: Config = read_config(&deps.storage)?;

        match from_binary(&msg)? {
            Cw20HookMsg::DepositReward { rewards } => {
                // only reward token contract can execute this message
                if config.nebula_token != deps.api.canonical_address(&env.message.sender)? {
                    return Err(StdError::unauthorized());
                }

                let mut rewards_amount = Uint128::zero();
                for (_, amount) in rewards.iter() {
                    rewards_amount += *amount;
                }

                if rewards_amount != cw20_msg.amount {
                    return Err(StdError::generic_err("rewards amount miss matched"));
                }

                deposit_reward(deps, rewards, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

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

                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                        spender: basket_contract.clone(),
                        amount: asset.amount,
                        expires: None,
                    })?,
                    send: vec![],
                }));
            }
        }
    }

    // mint basket token
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: basket_contract.clone(),
        msg: to_binary(&ClusterHandleMsg::Mint {
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
    let mut messages = vec![];
    let contract = env.contract.address.clone();

    let cfg: Config = read_config(&deps.storage)?;

    match asset.info {
        AssetInfo::Token { .. } => return Err(StdError::generic_err("not native token")),
        AssetInfo::NativeToken { ref denom } => {
            if denom.clone() != cfg.base_denom {
                return Err(StdError::generic_err("wrong base denom"));
            }
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
        send: vec![],
    }));

    // record pool state difference
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.clone(),
        msg: to_binary(&HandleMsg::RecordTerraswapImpact {
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
        msg: to_binary(&HandleMsg::RedeemAll {
            basket_contract: basket_contract.clone(),
            basket_token,
        })?,
        send: vec![],
    }));

    // send all
    // messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: contract_addr.clone(),
    //     msg: to_binary(&HandleMsg::SendAll {
    //         asset_infos: ,
    //         send_to: env.message.sender,
    //     })?,
    //     send: vec![],
    // }));
    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}

pub fn record_terraswap_impact<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
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
        // true_denom = true_bsk / fair_value
        // true_bsk = prod / true_denom
        // true_denom = prod / true_denom / fair_value
        // true_denom = sqrt(prod / fair_value)
        return FPDecimal::_pow(prod / fair_value, FPDecimal::one().div(2i128));

        let true_denom = FPDecimal::_pow(prod / fair_value, FPDecimal::one().div(2i128));
        return (amt_denom - true_denom).abs();
    }

    // if positive -> this arb moved us closer to fair value
    let imb0 = terraswap_imbalance(&pool_before.assets.to_vec(), fair_value);
    let imb1 = terraswap_imbalance(&pool_now.assets.to_vec(), fair_value);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("fair_value", fair_value), log("imbalance_diff", imb0 - imb1), log("imb0", imb0), log("imb1", imb1)],
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

        messages.push(asset.into_msg(&deps, env.contract.address.clone(), send_to.clone())?);
    }

    Ok(HandleResponse {
        messages,
        log: vec![log("action", "send_all")],
        data: None,
    })
}

pub fn redeem_all<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_contract: &HumanAddr,
    basket_token: &HumanAddr,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let amt_bsk = query_token_balance(deps, &basket_contract, &env.contract.address)?;

    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: basket_token.clone(),
                msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                    spender: basket_contract.clone(),
                    amount: amt_bsk,
                    expires: None,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: basket_contract.clone(),
                msg: to_binary(&ClusterHandleMsg::Burn {
                    max_tokens: amt_bsk,
                    asset_amounts: None,
                })?,
                send: vec![],
            }),
        ],
        log: vec![log("action", "redeem_all")],
        data: None,
    })
}

pub fn new_penalty_period<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    increment_n(&mut deps.storage)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "new_penalty_period")],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        terraswap_factory: deps.api.human_address(&state.terraswap_factory)?,
        nebula_token: deps.api.human_address(&state.nebula_token)?,
        base_denom: state.base_denom,
        owner: state.owner,
    };

    Ok(resp)
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> MigrateResult {
    Ok(MigrateResponse::default())
}
