use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::rebalancers::{assert_cluster_exists, get_cluster_state};
use crate::state::{read_config, record_contribution, Config};

use nebula_protocol::proxy::{ExecuteMsg, PoolType};

use astroport::pair::PoolResponse as AstroportPoolResponse;
use astroport::pair::QueryMsg as AstroportQueryMsg;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::{Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg};
use astroport::querier::{query_balance, query_pair_info, query_token_balance};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{ClusterStateResponse, QueryMsg as ClusterQueryMsg};

use crate::error::ContractError;
use cluster_math::FPDecimal;
use std::str::FromStr;

/// ## Description
/// Queries the given CT-UST pair info from Astroport.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **cluster_token** is a reference to an object of type [`Addr`].
pub fn get_pair_info(deps: Deps, cluster_token: &Addr) -> StdResult<PairInfo> {
    let config: Config = read_config(deps.storage)?;
    let astroport_factory_raw = config.astroport_factory;
    query_pair_info(
        &deps.querier,
        astroport_factory_raw,
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom,
            },
            AssetInfo::Token {
                contract_addr: cluster_token.clone(),
            },
        ],
    )
}

/// ## Description
/// Executes the create operation and uses cluster tokens (CT) to arbitrage on Astroport.
/// #### Assets -> UST
/// 1. Mint cluster tokens (CT) from the provided assets
/// 2. Swap all cluster tokens to UST on Astroport
/// 3. Record difference / change in Astroport pool before and after the swap
/// 4. Send all UST to the arbitrageur
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cluster_contract** is an object of type [`String`] which is the address of
///     a cluster contract.
///
/// - **assets** is a reference to an array containing objects of type [`Asset`] which is a list
///     of assets used to mint cluster tokens for arbitraging.
///
/// - **min_ust** is an object of type [`Option<Uint128>`] which is the minimum return amount
///     of UST required when swapping on Astroport.
pub fn arb_cluster_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    assets: &[Asset],
    min_ust: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;
    // Check if the provided address is an active cluster
    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address;

    let cfg: Config = read_config(deps.storage)?;

    // Get the cluster token contract address
    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;
    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    // Retrieve CT-UST pair info
    let pair_info = get_pair_info(deps.as_ref(), &cluster_token)?;

    // Transfer all asset tokens into this incentives contract
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

    // Performs 'Create' on the cluster contract minting cluster tokens from the provided assets
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedCreate {
            rebalancer: info.sender.clone(),
            cluster_contract: validated_cluster_contract.clone(),
            asset_amounts: assets.to_vec(),
            min_tokens: None,
        })?,
        funds: vec![],
    }));

    // Arbitrage on Astroport
    // Swap all minted cluster tokens to UST
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SwapAll {
            astroport_pair: pair_info.contract_addr.clone(),
            cluster_token,
            to_ust: true, // how about changing this to to_base
            min_return: min_ust,
        })?,
        funds: vec![],
    }));

    // Record Astroport pool state difference between before and after the arbitrage
    // This records the arbitrageur contribution used to compute Nebula token rewards
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
            arbitrageur: info.sender.clone(),
            astroport_pair: pair_info.contract_addr.clone(),
            cluster_contract: validated_cluster_contract,
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_binary(&AstroportQueryMsg::Pool {})?,
            }))?,
        })?,
        funds: vec![],
    }));

    // Send all UST from the incentives contract to the sender
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos: vec![AssetInfo::NativeToken {
                denom: cfg.base_denom,
            }],
            send_to: info.sender.clone(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "arb_cluster_create"),
        attr("sender", info.sender.as_str()),
    ]))
}

/// ## Description
/// Executes arbitrage on Astroport to get cluster tokens (CT) and performs the redeem operation.
/// #### UST -> Assets
/// 1. Swap all sent UST to cluster tokens (CT) on Astroport
/// 2. Record difference / change in Astroport pool before and after the swap
/// 3. Redeem the cluster tokens into the cluster's inventory assets
/// 4. Send all the redeemed assets to the arbitrageur
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cluster_contract** is an object of type [String`] which is the address of
///     a cluster contract.
///
/// - **asset** is an object of type [`Asset`] which contains the amount of UST
///     used for arbitraging the CT-UST pair.
///
/// - **min_return** is an object of type [`Option<Uint32>`] which is the minimum return amount
///     of cluster tokens required when swapping on Astroport.
pub fn arb_cluster_redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cluster_contract: String,
    asset: Asset,
    min_cluster: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;

    assert_cluster_exists(deps.as_ref(), &validated_cluster_contract)?;

    let mut messages = vec![];
    let contract = env.contract.address;

    let cfg: Config = read_config(deps.storage)?;

    // Get the cluster token contract address
    let cluster_state = get_cluster_state(deps.as_ref(), &validated_cluster_contract)?;
    let cluster_token = deps
        .api
        .addr_validate(cluster_state.cluster_token.as_str())?;

    // Assert UST is sent to the incentives contract ready to be swapped
    match asset.info {
        AssetInfo::Token { .. } => {
            return Err(ContractError::Generic("Not native token".to_string()))
        }
        AssetInfo::NativeToken { ref denom } => {
            if denom.clone() != cfg.base_denom {
                return Err(ContractError::Generic("Wrong base denom".to_string()));
            }
        }
    };
    asset.assert_sent_native_token_balance(&info)?;

    // Retrieve CT-UST pair info
    let pair_info = get_pair_info(deps.as_ref(), &cluster_token)?;

    // Arbitrage on Astroport
    // Swap all received UST to CT
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SwapAll {
            astroport_pair: pair_info.contract_addr.clone(),
            cluster_token: cluster_token.clone(),
            to_ust: false,
            min_return: min_cluster,
        })?,
        funds: vec![],
    }));

    // Record Astroport pool state difference between before and after the arbitrage
    // This records the arbitrageur contribution used to compute Nebula token rewards
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
            arbitrageur: info.sender.clone(),
            astroport_pair: pair_info.contract_addr.clone(),
            cluster_contract: validated_cluster_contract.clone(),
            pool_before: deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_binary(&AstroportQueryMsg::Pool {})?,
            }))?,
        })?,
        funds: vec![],
    }));

    // Performs 'Redeem' on the cluster contract burning cluster tokens with pro-rata rates
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
            rebalancer: info.sender.clone(),
            cluster_contract: validated_cluster_contract,
            cluster_token,
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

    // Send all assets from the redeem operation from the incentives contract to the sender
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&ExecuteMsg::_SendAll {
            asset_infos,
            send_to: info.sender.clone(),
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "arb_cluster_redeem"),
        attr("sender", info.sender.as_str()),
    ]))
}

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
    env: Env,
    info: MessageInfo,
    arbitrageur: Addr,
    astroport_pair: Addr,
    cluster_contract: Addr,
    pool_before: AstroportPoolResponse,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != env.contract.address {
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
/// Arbitrage / Swap either all UST -> CT or all CT -> UST on the Astroport pool.
/// -- We can do this because this contract never holds any inventory between transactions.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **astroport_pair** is an object of type [`Addr`] which is the address of
///     the Astroport pair contract that the arbitrage is executed on.
///
/// - **cluster_contract** is an object of type [`Addr`] which is the address of
///     the cluster contract corresponding to the arbitrage.
///
/// - **to_ust** is an object of type [`bool`] which determines the swap direction.
///
/// - **min_return** is an object of type [`Option<Uint128>`] which is the minimum
///     return amount expected from the exchange.
///
/// ## Executor
/// Only this contract can execute this.
pub fn swap_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    astroport_pair: Addr,
    cluster_token: Addr,
    to_ust: bool,
    min_return: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    let config: Config = read_config(deps.storage)?;
    let mut messages = vec![];

    let mut logs = vec![
        attr("action", "swap_all"),
        attr("to_usd", to_ust.to_string()),
    ];

    if to_ust {
        // Swap CT -> UST
        // Query CT balance on this incentives contract
        let amount =
            query_token_balance(&deps.querier, cluster_token.clone(), env.contract.address)?;

        // Calculate the belief price
        // -- belief_price = provided_CT / expected_UST
        let belief_price = min_return.map(|expected_ust| Decimal::from_ratio(amount, expected_ust));

        // Swap CT -> UST on Astroport pair pool
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: astroport_pair.to_string(),
                amount,
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: Some(Decimal::zero()),
                    belief_price,
                    to: None,
                })?,
            })?,
            funds: vec![],
        }));
        logs.push(attr("amount", amount));
        logs.push(attr("addr", astroport_pair.to_string()));
    } else {
        // Swap UST -> CT
        // Query UST balance on this incentives contract
        let amount = query_balance(
            &deps.querier,
            env.contract.address,
            config.base_denom.to_string(),
        )?;

        // Set the input for Astroport to be UST
        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // Deduct tax first
        let amount = (swap_asset.deduct_tax(&deps.querier)?).amount;

        // Calculate the belief price
        // -- belief_price = provided_UST / expected_CT
        let belief_price = min_return.map(|expected_ct| Decimal::from_ratio(amount, expected_ct));

        // Swap UST -> CT on Astroport pair pool
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: astroport_pair.to_string(),
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    amount,
                    ..swap_asset
                },
                max_spread: Some(Decimal::zero()),
                belief_price,
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

/// ## Description
/// Send all specified assets to an address.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_infos** is a reference to an array containing objects of type [`AssetInfo`]
///     which is a list of assets to be transferred.
///
/// - **send_to** is an object of type [`Addr`] which is the address of the receiver.
///
/// ## Executor
/// Only this contract can execute this.
pub fn send_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_infos: &[AssetInfo],
    send_to: Addr,
) -> Result<Response, ContractError> {
    // Permission check
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages = vec![];

    for asset_info in asset_infos {
        // Get the asset amount that the contract is holding
        let asset = Asset {
            info: asset_info.clone(),
            amount: match asset_info {
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    contract_addr.clone(),
                    env.contract.address.clone(),
                )?,
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?
                }
            },
        };
        // Create a send message
        if asset.amount > Uint128::zero() {
            messages.push(asset.into_msg(&deps.querier, Addr::unchecked(send_to.clone()))?);
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![attr("action", "send_all")]))
}
