use std::str::FromStr;

use astroport::asset::{Asset, AssetInfo};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use cluster_math::FPDecimal;
use nebula_protocol::cluster::ExecuteMsg;
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;

use crate::contract::{query_cluster_state, validate_targets};
use crate::error::ContractError;
use crate::ext_query::{
    query_collector_contract_address, query_create_amount, query_redeem_amount,
};
use crate::state::{config_store, read_config};
use crate::state::{
    read_asset_balance, read_target_asset_data, store_asset_balance, store_target_asset_data,
};
use crate::util::vec_to_string;

/// Prices last 60s before they go from fresh to stale
const FRESH_TIMESPAN: u64 = 60;

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             name,
///             description,
///             cluster_token,
///             pricing_oracle,
///             target_oracle,
///             penalty,
///             target,
///         }** Updates general contract parameters.
///
/// - **ExecuteMsg::RebalanceCreate {
///             asset_amounts,
///             min_tokens,
///         }** Perform Create operation, i.e. mint the cluster tokens.
///
/// - **ExecuteMsg::RebalanceRedeem {
///             max_tokens,
///             asset_amounts,
///         }** Perform Redeem operation, i.e. burn the cluster tokens.
///
/// - **ExecuteMsg::UpdateTarget { target }** Updates the target weights of assets in the cluster.
///
/// - **ExecuteMsg::Decommission {}** Decommission the cluster.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            name,
            description,
            cluster_token,
            pricing_oracle,
            target_oracle,
            penalty,
            target,
        } => update_config(
            deps,
            env,
            info,
            owner,
            name,
            description,
            cluster_token,
            pricing_oracle,
            target_oracle,
            penalty,
            target,
        ),
        ExecuteMsg::RebalanceCreate {
            asset_amounts,
            min_tokens,
        } => create(deps, env, info, asset_amounts, min_tokens),
        ExecuteMsg::RebalanceRedeem {
            max_tokens,
            asset_amounts,
        } => receive_redeem(deps, env, info, max_tokens, asset_amounts),
        ExecuteMsg::UpdateTarget { target } => update_target(deps, env, info, &target),
        ExecuteMsg::Decommission {} => decommission(deps, info),
    }
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is a new owner address to update.
///
/// - **name** is an object of type [`Option<String>`] which is a new cluster name to update.
///
/// - **description** is an object of type [`Option<String>`] which is a new cluster description.
///
/// - **cluster_token** is an object of type [`Option<String>`] which is the address of
///     the new cluster token contract.
///
/// - **pricing_oracle** is an object of type [`Option<String>`] which is the pricing oracle
///     contract address.
///
/// - **target_oracle** is an object of type [`Option<String>`] which is the address allowed
///     to update the asset target weights.
///
/// - **penalty** is an object of type [`Option<String>`] which is the address of the
///     new penalty contract.
///
/// - **target** is an object of type [`Option<Vec<Asset>>`] which is the new target weights
///     of the cluster assets.
///
/// ## Executor
/// Only the owner can execute this.
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    name: Option<String>,
    description: Option<String>,
    cluster_token: Option<String>,
    pricing_oracle: Option<String>,
    target_oracle: Option<String>,
    penalty: Option<String>,
    target: Option<Vec<Asset>>,
) -> Result<Response, ContractError> {
    let api = deps.api;

    // Update cluster config
    config_store(deps.storage).update(|mut config| {
        // Permission Check
        if config.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if let Some(owner) = owner {
            // Validate address format
            config.owner = api.addr_validate(owner.as_str())?;
        }

        if let Some(name) = name {
            config.name = name;
        }

        if let Some(description) = description {
            config.description = description;
        }

        if cluster_token.is_some() {
            // Validate address format, and transpose as `Option<Addr>`
            config.cluster_token = cluster_token
                .map(|x| api.addr_validate(x.as_str()))
                .transpose()?;
        }

        if let Some(pricing_oracle) = pricing_oracle {
            // Validate address format
            config.pricing_oracle = api.addr_validate(pricing_oracle.as_str())?;
        }

        if let Some(target_oracle) = target_oracle {
            // Validate address format
            config.target_oracle = api.addr_validate(target_oracle.as_str())?;
        }

        if let Some(penalty) = penalty {
            // Validate address format
            config.penalty = api.addr_validate(penalty.as_str())?;
        }

        Ok(config)
    })?;

    // Update cluster target
    if let Some(target) = target {
        update_target(deps, env, info, &target)?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Mints cluster tokens from the asset amounts given.
/// If `min_tokens` is specified, throws error when there can only be less than
/// `min_tokens` minted from the assets.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_amounts** is an object of type [`Vec<Asset>`] which are the assets traded
///     for minting cluster tokens.
///
/// - **min_tokens** is an object of type [`Option<Uint128>`] which is the required
///     minimum amount of minted cluster tokens.
pub fn create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_amounts: Vec<Asset>,
    min_tokens: Option<Uint128>,
) -> Result<Response, ContractError> {
    // Check `asset_amounts` for duplicate and unsupported assets
    if validate_targets(
        deps.querier,
        &env,
        asset_amounts.iter().map(|a| a.info.clone()).collect(),
    )
    .is_err()
    {
        return Err(ContractError::InvalidAssets {});
    }

    let cfg = read_config(deps.storage)?;

    if !cfg.active {
        // Cannot perform create operation on decommissioned clusters
        return Err(ContractError::ClusterAlreadyDecommissioned {});
    }

    // Retrieve the cluster state
    let cluster_state = query_cluster_state(
        deps.as_ref(),
        env.contract.address.as_ref(),
        env.block.time.seconds() - FRESH_TIMESPAN,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let inv = cluster_state.inv;
    let target = cluster_state.target;

    let target_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let mut native_coin_denoms_iter = target_infos
        .iter()
        .filter(|info| info.is_native_token())
        .map(|info| {
            match info {
                AssetInfo::NativeToken { denom } => Ok(denom.clone()),
                _ => Err(ContractError::Generic(
                    "Filtered list cannot contain non-native denoms".to_string(),
                )),
            }
            .unwrap()
        });

    let target_weights = target.iter().map(|x| x.amount).collect::<Vec<_>>();

    let cluster_token = cluster_state.cluster_token;

    // Vector to store create asset weights
    let mut asset_weights = vec![Uint128::zero(); target_infos.len()];

    let mut messages = vec![];

    // Return an error if assets not in target are sent to the create function
    for coin in info.funds.iter() {
        if !native_coin_denoms_iter.any(|x| x == coin.denom) {
            return Err(ContractError::Generic(
                "Unsupported assets were sent to the create function".to_string(),
            ));
        }
    }

    // Verify asset transfers and update cluster inventory balance
    for (i, asset_info) in target_infos.iter().enumerate() {
        for asset in asset_amounts.iter() {
            // Match each provided asset with the its target
            if asset.info.clone() == asset_info.clone() {
                // Verify the provided non-zero asset amount does not have target of zero
                if target_weights[i] == Uint128::zero() && asset.amount > Uint128::zero() {
                    return Err(ContractError::Generic(
                        format!("Cannot call create with non-zero asset amount when target weight is zero for asset {}", asset.info),
                    ));
                };

                asset_weights[i] = asset.amount;

                // Transfer assets from sender to this cluster contract
                if let AssetInfo::Token { contract_addr, .. } = &asset.info {
                    // Execute the asset CW20 contract to transfer the asset amount
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount: asset.amount,
                        })?,
                        funds: vec![],
                    }));

                    // Update asset inventory balance in the cluster
                    update_asset_balance(deps.storage, contract_addr.as_ref(), asset.amount, true)?;
                } else if let AssetInfo::NativeToken { denom } = &asset.info {
                    // Validate that native token balance is correct
                    asset.assert_sent_native_token_balance(&info)?;

                    // Update asset inventory balance in cluster
                    update_asset_balance(deps.storage, denom, asset.amount, true)?;
                }
                break;
            }
        }
    }

    let create_asset_amounts = asset_weights.clone();

    // Keep track of the cluster token amount minted to the sender
    let mint_amount_to_sender;

    // Mint cluster tokens and deduct protocol fees
    let mut extra_logs = vec![];
    // If cluster has been initialized
    if !cluster_token_supply.is_zero() {
        // Query cluster token amounts from a normal mint
        let create_response = query_create_amount(
            &deps.querier,
            &cfg.penalty,
            env.block.height,
            cluster_token_supply,
            inv.clone(),
            create_asset_amounts.clone(),
            prices.clone(),
            target_weights.clone(),
        )?;
        let create_amount = create_response.create_tokens;

        // Retrieve collector contract and fee rate
        let (collector_address, fee_rate) =
            query_collector_contract_address(&deps.querier, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&fee_rate)?;

        // Calculate the cluster token amount for sender and fee amount
        // mint_to_sender = mint_total * (1 - fee_rate)
        // protocol_fee = mint_total - mint_to_sender == mint_total * fee_rate
        let _mint_to_sender: u128 =
            (FPDecimal::from(create_amount.u128()) * (FPDecimal::one() - fee_rate)).into();
        mint_amount_to_sender = Uint128::from(_mint_to_sender);
        let protocol_fee = create_amount.checked_sub(mint_amount_to_sender)?;

        // Update penalty contract states
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.penalty.to_string(),
            msg: to_binary(&PenaltyExecuteMsg::PenaltyCreate {
                block_height: env.block.height,
                cluster_token_supply,
                inventory: inv,
                create_asset_amounts,
                asset_prices: prices,
                target_weights,
            })?,
            funds: vec![],
        }));

        // Mint cluster tokens of fee amount to the collector contract
        if !protocol_fee.is_zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: protocol_fee,
                    recipient: collector_address,
                })?,
                funds: vec![],
            }));
        }

        extra_logs = create_response.attributes;
        extra_logs.push(attr("fee_amt", protocol_fee))
    } else {
        // Cluster has no cluster tokens -- cluster is empty and needs to be initialized.
        // Attempt to initialize it with `min_tokens` as the number of cluster tokens
        // and the mint cluster c as the initial assets
        // c is required to be in ratio with the target weights
        if let Some(proposed_mint_total) = min_tokens {
            let mut val = 0;
            // Check if ratios, between each asset amount and its target weight, are all the same
            for i in 0..create_asset_amounts.len() {
                if (create_asset_amounts[i].u128() % target_weights[i].u128() != 0)
                    || create_asset_amounts[i] == Uint128::zero()
                {
                    return Err(ContractError::Generic(format!(
                        "Initial cluster assets must be a nonzero multiple of target weights at index {}",
                        i
                    )));
                }

                let div = create_asset_amounts[i].u128() / target_weights[i].u128();
                if val == 0 {
                    val = div;
                }

                if div != val {
                    return Err(ContractError::Generic(format!(
                        "Initial cluster assets have weight invariant at index {}",
                        i
                    )));
                }
            }

            // Set the cluster token mint amount to the `min_tokens`
            mint_amount_to_sender = proposed_mint_total;
        } else {
            return Err(ContractError::Generic(
                "Cluster is uninitialized. To initialize it with your mint cluster, \
                provide min_tokens as the amount of cluster tokens you want to start with."
                    .to_string(),
            ));
        }
    }

    // Validate that the mint amount is at least `min_tokens`
    if let Some(min_tokens) = min_tokens {
        if mint_amount_to_sender < min_tokens {
            return Err(ContractError::BelowMinTokens(
                mint_amount_to_sender,
                min_tokens,
            ));
        }
    }

    // Mint and send cluster tokens to the sender
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token,
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            amount: mint_amount_to_sender,
            recipient: info.sender.to_string(),
        })?,
        funds: vec![],
    }));

    let mut logs = vec![
        attr("action", "mint"),
        attr("sender", info.sender.to_string()),
        attr("mint_to_sender", mint_amount_to_sender),
    ];
    logs.extend(extra_logs);

    Ok(Response::new().add_messages(messages).add_attributes(logs))
}

/// ## Description
/// Receives cluster tokens which are burned for assets according to the given
/// `asset_weights` and cluster penalty parameter. The corresponding assets are
/// taken from the cluster inventory with the cluster token discount as rewards
/// based on whether the assets are moved towards/away from the target.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **max_tokens** is an object of type [`Uint128`] which is the required
///     maximum amount of cluster tokens allowed to burn.
///
/// - **asset_amounts** is an object of type [`Option<Vec<Asset>>`] which are the assets amount
///     the sender wishes to receive.
pub fn receive_redeem(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> Result<Response, ContractError> {
    let sender = info.sender;

    let cfg = read_config(deps.storage)?;

    // If cluster is not active, must do pro rata redeem
    if !cfg.active && asset_amounts.is_some() {
        return Err(ContractError::Generic(
            "Cannot call non pro-rata redeem on a decommissioned cluster".to_string(),
        ));
    }

    let asset_amounts = if !cfg.active { None } else { asset_amounts };

    let cluster_token = cfg
        .cluster_token
        .ok_or(ContractError::ClusterTokenNotSet {})?;

    // Use min as stale threshold if pro-rata redeem
    // - custom redeem: need asset prices to convert cluster tokens to assets
    // - pro-rata redeem: convert cluster tokens to assets based on the asset ratio
    //      in the current inventory
    let stale_threshold = match asset_amounts {
        Some(_) => env.block.time.seconds() - FRESH_TIMESPAN,
        None => u64::MIN,
    };

    // Retrieve the cluster state
    let cluster_state = query_cluster_state(
        deps.as_ref(),
        env.contract.address.as_ref(),
        stale_threshold,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let inv = cluster_state.inv;
    let target = cluster_state.target;

    let asset_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount).collect::<Vec<_>>();

    let asset_amounts: Vec<Uint128> = match &asset_amounts {
        Some(weights) => {
            let mut vec: Vec<Uint128> = vec![Uint128::zero(); asset_infos.len()];
            for i in 0..asset_infos.len() {
                for weight in weights {
                    if weight.info == asset_infos[i] {
                        vec[i] = weight.amount;
                        break;
                    }
                }
            }
            vec
        }
        None => vec![],
    };

    // Retrieve collector contract and fee rate
    let (collector_address, fee_rate) =
        query_collector_contract_address(&deps.querier, &cfg.factory)?;

    let fee_rate: FPDecimal = FPDecimal::from_str(&fee_rate)?;
    let keep_rate: FPDecimal = FPDecimal::one() - fee_rate;

    let _token_cap: u128 = (FPDecimal::from(max_tokens.u128()) * keep_rate).into();
    let token_cap: Uint128 = Uint128::from(_token_cap);

    // Query cluster token amounts burned with the maximum as `token_cap`
    let redeem_response = query_redeem_amount(
        &deps.querier,
        &cfg.penalty,
        env.block.height,
        cluster_token_supply,
        inv.clone(),
        token_cap,
        asset_amounts.clone(),
        prices.clone(),
        target_weights.clone(),
    )?;

    // Maximum redeem amount after deducting fees.
    let redeem_totals = redeem_response.redeem_assets;

    // Sanity check if token_cost is exceeding max_tokens
    let _token_cost: FPDecimal = FPDecimal::from(redeem_response.token_cost.u128()) / keep_rate;
    let mut token_cost: u128 = _token_cost.into();
    if FPDecimal::from(token_cost) != _token_cost {
        token_cost += 1u128;
    }

    let token_cost: Uint128 = Uint128::from(token_cost);
    if token_cost > max_tokens {
        return Err(ContractError::AboveMaxTokens(token_cost, max_tokens));
    }

    // Send `redeem_totals`, the assets from burning cluster tokens, to sender
    let mut messages: Vec<CosmosMsg> = redeem_totals
        .iter()
        .zip(asset_infos.iter())
        .filter(|(amt, _asset)| !amt.is_zero()) // remove 0 amounts
        .map(|(amt, asset_info)| {
            if let AssetInfo::Token { contract_addr, .. } = &asset_info {
                update_asset_balance(deps.storage, contract_addr.as_ref(), *amt, false)?;
            } else if let AssetInfo::NativeToken { denom } = &asset_info {
                update_asset_balance(deps.storage, denom, *amt, false)?;
            }
            let asset = Asset {
                info: asset_info.clone(),
                amount: *amt,
            };

            match asset.into_msg(&deps.querier, sender.clone()) {
                Ok(msg) => Ok(msg),
                Err(e) => Err(ContractError::Std(e)),
            }
        })
        .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

    // Compute fee based on the actual redeem amount `token_cost`
    let _fee_amt: FPDecimal = FPDecimal::from(token_cost.u128()) * fee_rate;
    let mut fee_amt: u128 = _fee_amt.into();
    if FPDecimal::from(fee_amt) != _fee_amt {
        fee_amt += 1
    }

    // Send fee to collector contract from allowance
    let fee_amt: Uint128 = Uint128::from(fee_amt);
    if !fee_amt.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: sender.to_string(),
                amount: fee_amt,
                recipient: collector_address,
            })?,
            funds: vec![],
        }));
    }

    // Burn the rest of the redeem amount from allowance
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
            owner: sender.to_string(),
            amount: token_cost.checked_sub(fee_amt)?,
        })?,
        funds: vec![],
    }));

    // Afterwards, notify the penalty contract that this update happened, so
    // the penalty contract can make stateful updates
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.penalty.to_string(),
        msg: to_binary(&PenaltyExecuteMsg::PenaltyRedeem {
            block_height: env.block.height,
            cluster_token_supply,
            inventory: inv,
            max_tokens,
            redeem_asset_amounts: asset_amounts.clone(),
            asset_prices: prices,
            target_weights,
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attributes(
        vec![
            vec![
                attr("action", "receive:burn"),
                attr("sender", sender),
                attr("burn_amount", token_cost.checked_sub(fee_amt)?.to_string()),
                attr("token_cost", token_cost),
                attr("kept_as_fee", fee_amt),
                attr("asset_amounts", vec_to_string(&asset_amounts)),
                attr("redeem_totals", vec_to_string(&redeem_totals)),
            ],
            redeem_response.attributes,
        ]
        .concat(),
    ))
}

/// ## Description
/// Updates the specific asset balance / inventory stored in the contract
///
/// ## Params
/// - **storage** is a mutable reference of an object implementing trait [`Storage`].
///
/// - **asset_id** is a reference to an object of type [`String`] which is
///     the asset to update.
///
/// - **amount** is an object of type [`Uint128`] which is an amount to update.
///
/// - **mint** is an object of type [`bool`] which specifies whether an
///     operation is mint or burn.
fn update_asset_balance(
    storage: &mut dyn Storage,
    asset_id: &str,
    amount: Uint128,
    mint: bool,
) -> Result<(), ContractError> {
    //  Get the asset balance of the cluster contract corresponding to `asset_id`
    let mut asset_amount = match read_asset_balance(storage, asset_id) {
        Ok(amount) => amount,
        Err(_) => Uint128::zero(),
    };

    // If the operation is mint, increase the asset balance with `amount`.
    // Otherwise, deduct the asset balance with `amount`
    match mint {
        true => asset_amount = asset_amount.checked_add(amount)?,
        false => asset_amount = asset_amount.checked_sub(amount)?,
    };

    // Save the new asset balance
    store_asset_balance(storage, asset_id, &asset_amount)?;
    Ok(())
}

/// ## Description
/// Changes the cluster target weights for different assets to the given
/// target weights and saves it. The ordering of the target weights is
/// determined by the given assets.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **target** is a reference to an object of type [`Vec<Asset>`] which is a new
///     asset target weights to update.
///
/// ## Executor
/// Only the owner or the target oracle address can execute this.
pub fn update_target(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    target: &[Asset],
) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;
    if cfg.cluster_token.is_none() {
        return Err(ContractError::ClusterTokenNotSet {});
    }

    // Can only update active cluster
    if !cfg.active {
        return Err(ContractError::ClusterAlreadyDecommissioned {});
    }

    // Permission check - can only be called by either the owner or target oracle address
    if (info.sender != cfg.owner) && (info.sender != cfg.target_oracle) {
        return Err(ContractError::Unauthorized {});
    }

    let mut asset_data = target.to_owned();

    // Create new vectors for logging and validation purpose
    // `update_asset_infos` contains the list of new assets
    // `update_target_weights` contains the list of weights for each new assets
    let (mut updated_asset_infos, mut updated_target_weights): (Vec<AssetInfo>, Vec<Uint128>) =
        asset_data
            .iter()
            .map(|x| (x.info.clone(), x.amount))
            .unzip();

    // Check `updated_asset_infos` for duplicate and unsupported assets
    if validate_targets(deps.querier, &env, updated_asset_infos.clone()).is_err() {
        return Err(ContractError::InvalidAssets {});
    }

    // Load previous assets & target
    let (prev_assets, prev_target): (Vec<AssetInfo>, Vec<Uint128>) =
        read_target_asset_data(deps.storage)?
            .iter()
            .map(|x| (x.info.clone(), x.amount))
            .unzip();

    // When previous assets are not found,
    // then set that not found item target to zero
    for prev_asset in prev_assets.iter() {
        let inv_balance = match prev_asset {
            AssetInfo::Token { contract_addr } => {
                read_asset_balance(deps.storage, contract_addr.as_ref())
            }
            AssetInfo::NativeToken { denom } => read_asset_balance(deps.storage, denom),
        }?;
        if !inv_balance.is_zero() && !updated_asset_infos.contains(prev_asset) {
            let asset_elem = Asset {
                info: prev_asset.clone(),
                amount: Uint128::zero(),
            };

            asset_data.push(asset_elem.clone());
            updated_asset_infos.push(asset_elem.info);
            updated_target_weights.push(asset_elem.amount);
        }
    }

    store_target_asset_data(deps.storage, &asset_data)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "reset_target"),
        attr("prev_assets", vec_to_string(&prev_assets)),
        attr("prev_targets", vec_to_string(&prev_target)),
        attr("updated_assets", vec_to_string(&updated_asset_infos)),
        attr("updated_targets", vec_to_string(&updated_target_weights)),
    ]))
}

/// ## Description
/// Decommissions an active cluster, disabling mints, and only allowing
/// pro-rata redeems
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// ## Executor
/// Only the factory contract can execute this.
pub fn decommission(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;
    if cfg.cluster_token.is_none() {
        return Err(ContractError::ClusterTokenNotSet {});
    }
    // Permission check - can only be decommissioned by the factory contract
    if info.sender != cfg.factory {
        return Err(ContractError::Unauthorized {});
    }

    // Can only decommission an active cluster
    if !cfg.active {
        return Err(ContractError::ClusterAlreadyDecommissioned {});
    }

    // Update the cluster state to be decommissioned / inactive
    config_store(deps.storage).update(|mut config| -> StdResult<_> {
        config.active = false;

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![attr("action", "decommission_asset")]))
}
