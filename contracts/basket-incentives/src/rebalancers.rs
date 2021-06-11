use cosmwasm_std::{
    log, to_binary, Api, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    Querier, QueryRequest, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{read_config, record_contribution};
use nebula_protocol::incentives::{HandleMsg, PoolType};

use cw20::Cw20HandleMsg;
use nebula_protocol::cluster::{
    BasketStateResponse, HandleMsg as ClusterHandleMsg, QueryMsg as ClusterQueryMsg,
};

use terraswap::asset::{Asset, AssetInfo};
use terraswap::querier::query_token_balance;

use basket_math::{imbalance, int32_vec_to_fpdec, int_vec_to_fpdec, str_vec_to_fpdec};
use nebula_protocol::factory::ClusterExistsResponse;
use nebula_protocol::factory::QueryMsg::ClusterExists;

pub fn get_basket_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    cluster: &HumanAddr,
) -> StdResult<BasketStateResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cluster.clone(),
        msg: to_binary(&ClusterQueryMsg::BasketState {
            basket_contract_address: cluster.clone(),
        })?,
    }))
}

pub fn assert_cluster_exists<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    cluster: &HumanAddr,
) -> StdResult<bool> {
    let cfg = read_config(&deps.storage)?;
    let res: ClusterExistsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.factory,
        msg: to_binary(&ClusterExists {
            contract_addr: cluster.clone(),
        })?,
    }))?;

    if res.exists {
        Ok(true)
    } else {
        Err(StdError::generic_err("specified does not exist"))
    }
}

pub fn basket_imbalance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_contract: &HumanAddr,
) -> StdResult<Uint128> {
    let contract_state = get_basket_state(deps, basket_contract)?;

    let i = int_vec_to_fpdec(&contract_state.inv);
    let p = str_vec_to_fpdec(&contract_state.prices)?;
    let w = int32_vec_to_fpdec(&contract_state.target);
    Ok(Uint128(imbalance(&i, &p, &w).into()))
}

pub fn record_rebalancer_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    rebalancer: HumanAddr,
    basket_contract: HumanAddr,
    original_imbalance: Uint128,
) -> HandleResult {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let new_imbalance = basket_imbalance(deps, &basket_contract)?;
    let mut contribution = Uint128::zero();

    if original_imbalance > new_imbalance {
        contribution = (original_imbalance - new_imbalance)?;

        record_contribution(
            deps,
            &rebalancer,
            PoolType::REBALANCER,
            &basket_contract,
            contribution,
        )?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "record_rebalancer_rewards"),
            log("rebalancer_imbalance_fixed", contribution),
        ],
        data: None,
    })
}

pub fn internal_rewarded_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    rebalancer: HumanAddr,
    basket_contract: HumanAddr,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let original_imbalance = basket_imbalance(deps, &basket_contract)?;

    let mut send = vec![];
    let mut messages = vec![];
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom } => send.push(Coin {
                denom,
                amount: asset.amount,
            }),
            AssetInfo::Token { contract_addr } => {
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

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: basket_contract.clone(),
        msg: to_binary(&ClusterHandleMsg::Mint {
            min_tokens,
            asset_amounts: asset_amounts.to_vec(),
        })?,
        send,
    }));
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::_RecordRebalancerRewards {
            rebalancer,
            basket_contract,
            original_imbalance,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        log: vec![log("action", "internal_rewarded_mint")],
        data: None,
    })
}

pub fn internal_rewarded_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    rebalancer: HumanAddr,
    basket_contract: HumanAddr,
    basket_token: HumanAddr,
    max_tokens: Option<Uint128>,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    let max_tokens = match max_tokens {
        None => query_token_balance(deps, &basket_token, &env.contract.address)?,
        Some(tokens) => tokens,
    };

    let original_imbalance = basket_imbalance(deps, &basket_contract)?;

    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: basket_token.clone(),
                msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                    spender: basket_contract.clone(),
                    amount: max_tokens,
                    expires: None,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: basket_contract.clone(),
                msg: to_binary(&ClusterHandleMsg::Burn {
                    max_tokens,
                    asset_amounts,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_RecordRebalancerRewards {
                    rebalancer: rebalancer.clone(),
                    basket_contract,
                    original_imbalance,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: basket_token,
                    }],
                    send_to: rebalancer,
                })?,
                send: vec![],
            }),
        ],
        log: vec![log("action", "internal_rewarded_redeem")],
        data: None,
    })
}

pub fn mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_contract: HumanAddr,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<HandleResponse> {
    assert_cluster_exists(deps, &basket_contract)?;

    let basket_state = get_basket_state(deps, &basket_contract)?;
    let basket_token = basket_state.basket_token;

    let mut messages = vec![];

    // transfer all asset tokens into this
    // also prepare to transfer to basket contract
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom: _ } => {
                asset.clone().assert_sent_native_token_balance(&env)?;
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&Cw20HandleMsg::TransferFrom {
                        owner: env.message.sender.clone(),
                        recipient: env.contract.address.clone(),
                        amount: asset.amount,
                    })?,
                    send: vec![],
                }));
            }
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.clone(),
        msg: to_binary(&HandleMsg::_InternalRewardedMint {
            rebalancer: env.message.sender.clone(),
            basket_contract,
            asset_amounts: asset_amounts.to_vec(),
            min_tokens,
        })?,
        send: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::SendAll {
            asset_infos: vec![AssetInfo::Token {
                contract_addr: basket_token,
            }],
            send_to: env.message.sender,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        log: vec![log("action", "mint")],
        data: None,
    })
}

pub fn redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_contract: HumanAddr,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    assert_cluster_exists(deps, &basket_contract)?;

    let basket_state = get_basket_state(deps, &basket_contract)?;
    let basket_token = basket_state.basket_token;

    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: basket_token.clone(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    amount: max_tokens,
                    recipient: env.contract.address.clone(),
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    basket_contract,
                    basket_token,
                    max_tokens: Some(max_tokens),
                    asset_amounts,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::SendAll {
                    asset_infos: basket_state.assets,
                    send_to: env.message.sender,
                })?,
                send: vec![],
            }),
        ],
        log: vec![log("action", "internal_rewarded_redeem")],
        data: None,
    })
}
