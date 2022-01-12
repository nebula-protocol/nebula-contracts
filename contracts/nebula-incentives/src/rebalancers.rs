use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{read_config, record_contribution};
use nebula_protocol::incentives::{ExecuteMsg, PoolType};

use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{
    ClusterStateResponse, ExecuteMsg as ClusterExecuteMsg, QueryMsg as ClusterQueryMsg,
};

use astroport::asset::{Asset, AssetInfo};
use astroport::querier::query_token_balance;

use cluster_math::{imbalance, int_vec_to_fpdec, str_vec_to_fpdec};
use nebula_protocol::cluster_factory::ClusterExistsResponse;
use nebula_protocol::cluster_factory::QueryMsg::ClusterExists;
use std::cmp::min;

pub fn get_cluster_state(deps: Deps, cluster: &Addr) -> StdResult<ClusterStateResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cluster.to_string(),
        msg: to_binary(&ClusterQueryMsg::ClusterState {})?,
    }))
}

pub fn assert_cluster_exists(deps: Deps, cluster: &Addr) -> StdResult<bool> {
    let cfg = read_config(deps.storage)?;
    let res: ClusterExistsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.factory.to_string(),
        msg: to_binary(&ClusterExists {
            contract_addr: cluster.to_string(),
        })?,
    }))?;

    if res.exists {
        Ok(true)
    } else {
        Err(StdError::generic_err("specified cluster does not exist"))
    }
}

pub fn cluster_imbalance(deps: Deps, cluster_contract: &Addr) -> StdResult<Uint128> {
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
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    original_imbalance: Uint128,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let new_imbalance = cluster_imbalance(deps.as_ref(), &cluster_contract)?;
    let mut contribution = Uint128::zero();

    if original_imbalance > new_imbalance {
        contribution = original_imbalance.checked_sub(new_imbalance)?;

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

pub fn internal_rewarded_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let original_imbalance = cluster_imbalance(deps.as_ref(), &cluster_contract)?;
    let mut funds = vec![];
    let mut create_asset_amounts = vec![];
    let mut messages = vec![];
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom } => {
                let amount = (asset.clone().deduct_tax(&deps.querier)?).amount;

                let new_asset = Asset {
                    amount,
                    ..asset.clone()
                };

                create_asset_amounts.push(new_asset);
                funds.push(Coin {
                    denom: denom.clone(),
                    amount,
                });
            }
            AssetInfo::Token { contract_addr } => {
                create_asset_amounts.push(asset.clone());
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                        spender: cluster_contract.to_string(),
                        amount: asset.amount,
                        expires: None,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }
    funds.sort_by(|c1, c2| c1.denom.cmp(&c2.denom));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_contract.to_string(),
        msg: to_binary(&ClusterExecuteMsg::RebalanceCreate {
            min_tokens,
            asset_amounts: create_asset_amounts,
        })?,
        funds,
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
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
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    cluster_token: Addr,
    max_tokens: Option<Uint128>,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<Response> {
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    let max_tokens = match max_tokens {
        None => query_token_balance(
            &deps.querier,
            cluster_token.clone(),
            env.contract.address.clone(),
        )?,
        Some(tokens) => tokens,
    };

    let original_imbalance = cluster_imbalance(deps.as_ref(), &cluster_contract)?;

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: cluster_contract.to_string(),
                    amount: max_tokens,
                    expires: None,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_contract.to_string(),
                msg: to_binary(&ClusterExecuteMsg::RebalanceRedeem {
                    max_tokens,
                    asset_amounts,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: rebalancer.clone(),
                    cluster_contract,
                    original_imbalance,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
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

pub fn create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> StdResult<Response> {
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;

    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract)?;

    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;

    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    let mut messages = vec![];

    // transfer all asset tokens into this
    // also prepare to transfer to cluster contract
    for asset in asset_amounts {
        match asset.clone().info {
            AssetInfo::NativeToken { denom: _ } => {
                asset.clone().assert_sent_native_token_balance(&info)?;
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: asset.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedCreate {
            rebalancer: info.sender.clone(),
            cluster_contract: validated_cluster_contract,
            asset_amounts: asset_amounts.to_vec(),
            min_tokens,
        })?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos: vec![AssetInfo::Token {
                contract_addr: cluster_token,
            }],
            send_to: info.sender.clone(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "incentives_create"),
        attr("sender", info.sender.as_str()),
    ]))
}

pub fn redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<Response> {
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;

    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract)?;

    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;

    // Only alow pro-rata redeem if cluster is not active
    let asset_amounts = if !cluster_state.active {
        None
    } else {
        asset_amounts
    };
    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    let max_tokens = min(
        max_tokens,
        query_token_balance(&deps.querier, cluster_token.clone(), info.sender.clone())?,
    );

    let asset_infos = cluster_state
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    amount: max_tokens,
                    recipient: env.contract.address.to_string(),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: info.sender.clone(),
                    cluster_contract: validated_cluster_contract,
                    cluster_token,
                    max_tokens: Some(max_tokens),
                    asset_amounts,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos,
                    send_to: info.sender.clone(),
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "incentives_redeem"),
            attr("sender", info.sender.as_str()),
        ]))
}
