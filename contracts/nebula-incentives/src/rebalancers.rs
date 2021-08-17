use cosmwasm_std::{
    attr, to_binary, Coin, CosmosMsg, Deps, DepsMut, Env, HumanAddr, QueryRequest, Response,
    StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{read_config, record_contribution};
use nebula_protocol::incentives::{ExecuteMsg, PoolType};

use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{
    ClusterStateResponse, ExecuteMsg as ClusterExecuteMsg, QueryMsg as ClusterQueryMsg,
};

use terraswap::asset::{Asset, AssetInfo};
use terraswap::querier::query_token_balance;

use cluster_math::{imbalance, int_vec_to_fpdec, str_vec_to_fpdec};
use nebula_protocol::cluster_factory::ClusterExistsResponse;
use nebula_protocol::cluster_factory::QueryMsg::ClusterExists;
use std::cmp::min;

pub fn get_cluster_state(deps: Deps, cluster: &HumanAddr) -> StdResult<ClusterStateResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cluster.clone(),
        msg: to_binary(&ClusterQueryMsg::ClusterState {
            cluster_contract_address: cluster.clone(),
        })?,
    }))
}

pub fn assert_cluster_exists(deps: Deps, cluster: &HumanAddr) -> StdResult<bool> {
    let cfg = read_config(deps.storage)?;
    let res: ClusterExistsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.factory,
        msg: to_binary(&ClusterExists {
            contract_addr: cluster.clone(),
        })?,
    }))?;

    if res.exists {
        Ok(true)
    } else {
        Err(StdError::generic_err("specified cluster does not exist"))
    }
}

pub fn cluster_imbalance(deps: Deps, cluster_contract: &HumanAddr) -> StdResult<Uint128> {
    let cluster_state = get_cluster_state(deps, cluster_contract)?;

    let i = int_vec_to_fpdec(&cluster_state.inv);
    let p = str_vec_to_fpdec(&cluster_state.prices)?;

    let target_weights = cluster_state
        .target
        .iter()
        .map(|x| x.amount.clone())
        .collect::<Vec<_>>();

    let w = int_vec_to_fpdec(&target_weights);
    Ok(Uint128::new(imbalance(&i, &p, &w).into()))
}

pub fn record_rebalancer_rewards(
    deps: DepsMut,
    env: Env,
    rebalancer: HumanAddr,
    cluster_contract: HumanAddr,
    original_imbalance: Uint128,
) -> StdResult<Response> {
    if env.message.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let new_imbalance = cluster_imbalance(deps, &cluster_contract)?;
    let mut contribution = Uint128::zero();

    if original_imbalance > new_imbalance {
        contribution = (original_imbalance - new_imbalance)?;

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

pub fn internal_rewarded_mint(
    deps: DepsMut,
    env: Env,
    rebalancer: HumanAddr,
    cluster_contract: HumanAddr,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<Response> {
    if env.message.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let original_imbalance = cluster_imbalance(deps, &cluster_contract)?;
    let mut send = vec![];
    let mut mint_asset_amounts = vec![];
    let mut messages = vec![];
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom } => {
                let amount = (asset.clone().deduct_tax(&deps)?).amount;

                let new_asset = Asset {
                    amount,
                    ..asset.clone()
                };

                mint_asset_amounts.push(new_asset);
                send.push(Coin {
                    denom: denom.clone(),
                    amount,
                });
            }
            AssetInfo::Token { contract_addr } => {
                mint_asset_amounts.push(asset.clone());
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                        spender: cluster_contract.clone(),
                        amount: asset.amount,
                        expires: None,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }
    send.sort_by(|c1, c2| c1.denom.cmp(&c2.denom));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_contract.clone(),
        msg: to_binary(&ClusterExecuteMsg::Mint {
            min_tokens,
            asset_amounts: mint_asset_amounts,
        })?,
        send,
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
            rebalancer,
            cluster_contract,
            original_imbalance,
        })?,
        funds: vec![],
    }));
    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "internal_rewarded_mint")]))
}

pub fn internal_rewarded_redeem(
    deps: DepsMut,
    env: Env,
    rebalancer: HumanAddr,
    cluster_contract: HumanAddr,
    cluster_token: HumanAddr,
    max_tokens: Option<Uint128>,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<Response> {
    if env.message.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let max_tokens = match max_tokens {
        None => query_token_balance(deps, &cluster_token, &env.contract.address)?,
        Some(tokens) => tokens,
    };

    let original_imbalance = cluster_imbalance(deps, &cluster_contract)?;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.clone(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: cluster_contract.clone(),
                    amount: max_tokens,
                    expires: None,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_contract.clone(),
                msg: to_binary(&ClusterExecuteMsg::Burn {
                    max_tokens,
                    asset_amounts,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: rebalancer.clone(),
                    cluster_contract,
                    original_imbalance,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: cluster_token,
                    }],
                    send_to: rebalancer,
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![attr("action", "internal_rewarded_redeem")]))
}

pub fn mint(
    deps: DepsMut,
    env: Env,
    cluster_contract: HumanAddr,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<Response> {
    assert_cluster_exists(deps, &cluster_contract)?;

    let cluster_state = get_cluster_state(deps, &cluster_contract)?;

    let cluster_token = cluster_state.cluster_token;

    let mut messages = vec![];

    // transfer all asset tokens into this
    // also prepare to transfer to cluster contract
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom: _ } => {
                asset.clone().assert_sent_native_token_balance(&env)?;
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: env.message.sender.clone(),
                        recipient: env.contract.address.clone(),
                        amount: asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.clone(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedMint {
            rebalancer: env.message.sender.clone(),
            cluster_contract,
            asset_amounts: asset_amounts.to_vec(),
            min_tokens,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos: vec![AssetInfo::Token {
                contract_addr: cluster_token,
            }],
            send_to: env.message.sender,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "mint")]))
}

pub fn redeem(
    deps: DepsMut,
    env: Env,
    cluster_contract: HumanAddr,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<Response> {
    assert_cluster_exists(deps, &cluster_contract)?;

    let cluster_state = get_cluster_state(deps, &cluster_contract)?;

    // Only alow pro-rata redeem if cluster is not active
    let asset_amounts = if !cluster_state.active {
        None
    } else {
        asset_amounts
    };
    let cluster_token = cluster_state.cluster_token;

    let max_tokens = min(
        max_tokens,
        query_token_balance(deps, &cluster_token, &env.message.sender)?,
    );

    let asset_infos = cluster_state
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.clone(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    amount: max_tokens,
                    recipient: env.contract.address.clone(),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract,
                    cluster_token,
                    max_tokens: Some(max_tokens),
                    asset_amounts,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos,
                    send_to: env.message.sender,
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![attr("action", "internal_rewarded_redeem")]))
}
