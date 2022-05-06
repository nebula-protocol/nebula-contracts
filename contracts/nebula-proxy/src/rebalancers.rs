use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{read_config, Config};
use nebula_protocol::proxy::ExecuteMsg;

use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{
    ClusterStateResponse, ExecuteMsg as ClusterExecuteMsg, QueryMsg as ClusterQueryMsg,
};
use nebula_protocol::incentives::ExecuteMsg as IncentivesExecuteMsg;

use astroport::asset::{Asset, AssetInfo};
use astroport::querier::query_token_balance;

use crate::error::ContractError;
use nebula_protocol::cluster_factory::ClusterExistsResponse;
use nebula_protocol::cluster_factory::QueryMsg::ClusterExists;
use std::cmp::min;

/// ## Description
/// Returns the state of a cluster.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **cluster** is a reference to an object of type [`Addr`] which is
///     the address of a cluster.
pub fn get_cluster_state(deps: Deps, cluster: &Addr) -> StdResult<ClusterStateResponse> {
    // Query the cluster state
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cluster.to_string(),
        msg: to_binary(&ClusterQueryMsg::ClusterState {})?,
    }))
}

/// ## Description
/// Returns whether a specific cluster is an active cluster.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **cluster** is a reference to an object of type [`Addr`] which is
///     the address of a cluster.
pub fn assert_cluster_exists(
    deps: Deps,
    cluster: &Addr,
    cfg: &Config,
) -> Result<bool, ContractError> {
    // Query cluster status
    let res: ClusterExistsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.factory.to_string(),
        msg: to_binary(&ClusterExists {
            contract_addr: cluster.to_string(),
        })?,
    }))?;

    if res.exists {
        Ok(true)
    } else {
        Err(ContractError::Generic(
            "Specified cluster does not exist".to_string(),
        ))
    }
}

/// ## Description
/// Calls the actual create logic in a cluster contract used in both arbitraging and rebalancing.
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
/// - **incentives** is an object of type [`Option<Addr>`] which, if exists, is the
///     address of the incentives contract.
///
/// - **asset_amounts** is a reference to an array containing objects of type [`Asset`]
///     which is a list of assets offerred to mint cluster tokens.
///
/// - **min_tokens** is an object of type [`Option<Uint128>`] which is the minimum required
///     amount of cluster tokens minted from this create operation.
///
/// ## Executor
/// Only this contract can execute this.
#[allow(clippy::too_many_arguments)]
pub fn internal_rewarded_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    incentives: Option<Addr>,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // Perpare to transfer to the cluster contract
    let mut funds = vec![];
    let mut create_asset_amounts = vec![];
    let mut messages = vec![];
    for asset in asset_amounts {
        match asset.info.clone() {
            AssetInfo::NativeToken { denom } => {
                let amount = (asset.deduct_tax(&deps.querier)?).amount;

                let new_asset = Asset {
                    amount,
                    ..asset.clone()
                };

                create_asset_amounts.push(new_asset);
                funds.push(Coin { denom, amount });
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

    // Call cluster contract to perform the create operation
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_contract.to_string(),
        msg: to_binary(&ClusterExecuteMsg::RebalanceCreate {
            min_tokens,
            asset_amounts: create_asset_amounts,
        })?,
        funds,
    }));

    if let Some(incentives) = incentives {
        // Retrieve the current cluster inventory
        let original_inventory = get_cluster_state(deps.as_ref(), &cluster_contract)?.inv;

        // Record the change in the cluster imbalance for contribution rewards
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: incentives.to_string(),
            msg: to_binary(&IncentivesExecuteMsg::RecordRebalancerRewards {
                rebalancer,
                cluster_contract,
                original_inventory,
            })?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "internal_rewarded_mint")]))
}

/// ## Description
/// Calls the actual redeem logic in a cluster contract used in both arbitraging and rebalancing.
/// If `asset_amount` is not provided, pro-rata rate will be used.
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
/// - **cluster_token** is an object of type [`Addr`] which is the address of
///     the corresponding cluster token contract.
///
/// - **incentives** is an object of type [`Option<Addr>`] which, if exists, is the
///     address of the incentives contract.
///
/// - **max_tokens** is an object of type [`Option<Uint128>`] which is the maximum allowed
///     amount of cluster tokens to be burned from this create operation.
///
/// - **asset_amounts** is an object of type [`Option<Vec<Asset>>`] which are the assets amount
///     the rebalancer wishes to receive.
///
/// ## Executor
/// Only this contract can execute this.
#[allow(clippy::too_many_arguments)]
pub fn internal_rewarded_redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rebalancer: Addr,
    cluster_contract: Addr,
    cluster_token: Addr,
    incentives: Option<Addr>,
    max_tokens: Option<Uint128>,
    asset_amounts: Option<Vec<Asset>>,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // If `max_tokens` is not provided, query the CT balance of this contract
    let max_tokens = match max_tokens {
        None => query_token_balance(
            &deps.querier,
            cluster_token.clone(),
            env.contract.address.clone(),
        )?,
        Some(tokens) => tokens,
    };

    let mut messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
            spender: cluster_contract.to_string(),
            amount: max_tokens,
            expires: None,
        })?,
        funds: vec![],
    })];

    // Call cluster contract to perform the redeem operation
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_contract.to_string(),
        msg: to_binary(&ClusterExecuteMsg::RebalanceRedeem {
            max_tokens,
            asset_amounts,
        })?,
        funds: vec![],
    }));

    if let Some(incentives) = incentives {
        // Retrieve the current cluster inventory
        let original_inventory = get_cluster_state(deps.as_ref(), &cluster_contract)?.inv;

        // Record the change in the cluster imbalance for contribution rewards
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: incentives.to_string(),
            msg: to_binary(&IncentivesExecuteMsg::RecordRebalancerRewards {
                rebalancer: rebalancer.clone(),
                cluster_contract,
                original_inventory,
            })?,
            funds: vec![],
        }));
    }

    // Returns the remaining cluster tokens back to the rebalancer
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos: vec![AssetInfo::Token {
                contract_addr: cluster_token,
            }],
            send_to: rebalancer,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "internal_rewarded_redeem")]))
}

/// ## Description
/// Executes the create operation on a specific cluster.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cluster_contract** is an object of type [`String`] which is the address of
///     the cluster contract to perform the operation on.
///
/// - **asset_amounts** is a reference to an array containing objects of type [`Asset`] which is a list
///     of assets offerred to mint cluster tokens.
///
/// - **min_tokens** is an object of type [`Option<Uint128>`] which is the minimum required
///     amount of cluster tokens minted from this create operation.
pub fn create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    asset_amounts: &[Asset],
    min_tokens: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;

    let cfg = read_config(deps.storage)?;
    // Check if it is an active cluster
    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract, &cfg)?;

    // Get the cluster state
    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;

    // Validate address format
    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    let mut messages = vec![];

    // Transfer all asset tokens of specified amounts into this incentives contract
    for asset in asset_amounts {
        match asset.info.clone() {
            AssetInfo::NativeToken { denom: _ } => {
                asset.assert_sent_native_token_balance(&info)?;
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

    // Perform the create operation
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedCreate {
            rebalancer: info.sender.clone(),
            cluster_contract: validated_cluster_contract,
            incentives: cfg.incentives,
            asset_amounts: asset_amounts.to_vec(),
            min_tokens,
        })?,
        funds: vec![],
    }));

    // Send all minted CT back to the sender
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

/// ## Description
/// Executes the redeem operation on a specific cluster.
/// If `asset_amount` is not provided, pro-rata rate will be used.
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
/// - **cluster_token** is an object of type [`Addr`] which is the address of
///     the corresponding cluster token contract.
///
/// - **max_tokens** is an object of type [`Uint128`] which is the maximum allowed
///     amount of cluster tokens to be burned from this create operation.
///
/// - **asset_amounts** is an object of type [`Option<Vec<Asset>>`] which are the assets amount
///     the rebalancer wishes to receive.
pub fn redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;

    let cfg = read_config(deps.storage)?;
    // Check if it is an active cluster
    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract, &cfg)?;

    // Get the cluster state
    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;

    // Only alow pro-rata redeem if cluster is not active
    let asset_amounts = if !cluster_state.active {
        None
    } else {
        asset_amounts
    };

    // Validate address format
    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    // Set `max_tokens` to be the maximum between the provided `max_tokens` and the actual sender balance
    let max_tokens = min(
        max_tokens,
        query_token_balance(&deps.querier, cluster_token.clone(), info.sender.clone())?,
    );

    // Retrieve the list of assets in the cluster
    let asset_infos = cluster_state
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    Ok(Response::new()
        .add_messages(vec![
            // Transfer CT tokens of `max_tokens` into this incentives contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    amount: max_tokens,
                    recipient: env.contract.address.to_string(),
                })?,
                funds: vec![],
            }),
            // Perform the redeem operation
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: info.sender.clone(),
                    cluster_contract: validated_cluster_contract,
                    cluster_token,
                    incentives: cfg.incentives,
                    max_tokens: Some(max_tokens),
                    asset_amounts,
                })?,
                funds: vec![],
            }),
            // Send all assets returned from burning CT tokens to the sender
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
