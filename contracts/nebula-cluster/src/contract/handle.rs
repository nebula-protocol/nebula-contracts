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

// prices last 30s before they go from fresh to stale
const FRESH_TIMESPAN: u64 = 30;

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
        if config.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if let Some(owner) = owner {
            config.owner = api.addr_validate(owner.as_str())?;
        }

        if let Some(name) = name {
            config.name = name;
        }

        if let Some(description) = description {
            config.description = description;
        }

        if cluster_token.is_some() {
            config.cluster_token = cluster_token
                .map(|x| api.addr_validate(x.as_str()))
                .transpose()?;
        }

        if let Some(pricing_oracle) = pricing_oracle {
            config.pricing_oracle = api.addr_validate(pricing_oracle.as_str())?;
        }

        if let Some(target_oracle) = target_oracle {
            config.target_oracle = api.addr_validate(target_oracle.as_str())?;
        }

        if let Some(penalty) = penalty {
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

/*
    Changes the cluster target weights for different assets to the given
    target weights and saves it. The ordering of the target weights is
    determined by the given assets.
*/
pub fn update_target(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    target: &Vec<Asset>,
) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(ContractError::ClusterTokenNotSet {});
    }

    // check permission
    if (info.sender != cfg.owner) && (info.sender != cfg.target_oracle) {
        return Err(ContractError::Unauthorized {});
    }

    let mut asset_data = target.clone();

    // Create new vectors for logging and validation purpose
    // update_asset_infos contains the list of new assets
    // update_target_weights contains the list of weights for each new assets
    let (mut updated_asset_infos, mut updated_target_weights): (Vec<AssetInfo>, Vec<Uint128>) =
        asset_data
            .iter()
            .map(|x| (x.info.clone(), x.amount.clone()))
            .unzip();

    if validate_targets(deps.querier, &env, updated_asset_infos.clone()).is_err() {
        return Err(ContractError::InvalidAssets {});
    }

    // Load previous assets & target
    let (prev_assets, prev_target): (Vec<AssetInfo>, Vec<Uint128>) =
        read_target_asset_data(deps.storage)?
            .iter()
            .map(|x| (x.info.clone(), x.amount.clone()))
            .unzip();

    // When previous assets are not found,
    // then set that not found item target to zero
    for prev_asset in prev_assets.iter() {
        let inv_balance = match prev_asset {
            AssetInfo::Token { contract_addr } => {
                read_asset_balance(deps.storage, &contract_addr.to_string())
            }
            AssetInfo::NativeToken { denom } => read_asset_balance(deps.storage, denom),
        }?;
        if !inv_balance.is_zero() && !updated_asset_infos.contains(&prev_asset) {
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

/*
    Decommissions an active cluster, disabling mints, and only allowing
    pro-rata redeems
*/
pub fn decommission(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // allow removal / adding
    let cfg = read_config(deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(ContractError::ClusterTokenNotSet {});
    }
    // check permission for factory
    if info.sender != cfg.factory {
        return Err(ContractError::Unauthorized {});
    }

    // can only decommission an active cluster
    if !cfg.active {
        return Err(ContractError::ClusterAlreadyDecommissioned {});
    }

    config_store(deps.storage).update(|mut config| -> StdResult<_> {
        config.active = false;

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![attr("action", "decommission_asset")]))
}

/*
    Mint cluster tokens from the asset amounts given.
    Throws error if there can only be less than 'min_tokens' minted from the assets.
*/
pub fn create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_amounts: Vec<Asset>,
    min_tokens: Option<Uint128>,
) -> Result<Response, ContractError> {
    // check asset_amounts for duplicate and unsupported assets
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
        return Err(ContractError::ClusterAlreadyDecommissioned {});
    }

    let cluster_state = query_cluster_state(
        deps.as_ref(),
        &env.contract.address.to_string(),
        env.block.time.seconds() - FRESH_TIMESPAN,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let inv = cluster_state.inv;
    let target = cluster_state.target;

    let target_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let native_coin_denoms = target_infos
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
        })
        .collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount.clone()).collect::<Vec<_>>();

    let cluster_token = cluster_state.cluster_token;

    // Vector to store create asset weights
    let mut asset_weights = vec![Uint128::zero(); target_infos.len()];

    let mut messages = vec![];

    // Return an error if assets not in target are sent to the create function
    for coin in info.funds.iter() {
        if !native_coin_denoms.contains(&coin.denom) {
            return Err(ContractError::Generic(
                "Unsupported assets were sent to the create function".to_string(),
            ));
        }
    }

    // verify asset transfers and update cluster inventory balance
    for (i, asset_info) in target_infos.iter().enumerate() {
        for asset in asset_amounts.iter() {
            if asset.info.clone() == asset_info.clone() {
                if target_weights[i] == Uint128::zero() && asset.amount > Uint128::zero() {
                    return Err(ContractError::Generic(
                        format!("Cannot call create with non-zero asset amount when target weight is zero for asset {}", asset.info.to_string()),
                    ));
                };

                asset_weights[i] = asset.amount;

                // transfer assets from sender
                if let AssetInfo::Token { contract_addr, .. } = &asset.info {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount: asset.amount,
                        })?,
                        funds: vec![],
                    }));

                    // update asset inventory balance in cluster
                    update_asset_balance(
                        deps.storage,
                        &contract_addr.to_string(),
                        asset.amount,
                        true,
                    )?;
                } else if let AssetInfo::NativeToken { denom } = &asset.info {
                    // validate that native token balance is correct
                    asset.assert_sent_native_token_balance(&info)?;

                    // update asset inventory balance in cluster
                    update_asset_balance(deps.storage, denom, asset.amount, true)?;
                }
                break;
            }
        }
    }

    let create_asset_amounts = asset_weights.clone();

    let mint_amount_to_sender;

    // mint cluster tokens and deduct protocol fees
    let mut extra_logs = vec![];
    if !cluster_token_supply.is_zero() {
        // cluster has been initialized
        // perform a normal mint
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

        let (collector_address, fee_rate) =
            query_collector_contract_address(&deps.querier, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&fee_rate)?;

        // calculate fee amount
        // mint_to_sender = mint_total * (1 - fee_rate)
        // protocol_fee = mint_total - mint_to_sender == mint_total * fee_rate
        let _mint_to_sender: u128 =
            (FPDecimal::from(create_amount.u128()) * (FPDecimal::one() - fee_rate)).into();
        mint_amount_to_sender = Uint128::from(_mint_to_sender);
        let protocol_fee = create_amount.checked_sub(mint_amount_to_sender)?;

        // update penalty contract states
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

        // mint cluster tokens
        if !protocol_fee.is_zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: protocol_fee,
                    recipient: collector_address.to_string(),
                })?,
                funds: vec![],
            }));
        }

        extra_logs = create_response.attributes;
        extra_logs.push(attr("fee_amt", protocol_fee))
    } else {
        // cluster has no cluster tokens -- cluster is empty and needs to be initialized
        // attempt to initialize it with min_tokens as the number of cluster tokens
        // and the mint cluster c as the initial assets
        // c is required to be in ratio with the target weights
        if let Some(proposed_mint_total) = min_tokens {
            let mut val = 0;
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

            mint_amount_to_sender = proposed_mint_total;
        } else {
            return Err(ContractError::Generic(
                "Cluster is uninitialized. To initialize it with your mint cluster, \
                provide min_tokens as the amount of cluster tokens you want to start with."
                    .to_string(),
            ));
        }
    }

    if let Some(min_tokens) = min_tokens {
        if mint_amount_to_sender < min_tokens {
            return Err(ContractError::BelowMinTokens(
                mint_amount_to_sender,
                min_tokens,
            ));
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            amount: mint_amount_to_sender,
            recipient: info.sender.to_string(),
        })?,
        funds: vec![],
    }));

    let mut logs = vec![
        attr("action", "mint"),
        attr("sender", &info.sender.to_string()),
        attr("mint_to_sender", mint_amount_to_sender),
    ];
    logs.extend(extra_logs);

    // mint and send number of tokens to user
    Ok(Response::new().add_messages(messages).add_attributes(logs))
}

/*
    Receives cluster tokens which are burned for assets according to
    the given asset_weights and cluster penalty parameter. The corresponding
    assets are taken from the cluster inventory and sent back to the user
    along with any rewards based on whether the assets are moved towards/away
    from the target.
*/
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
        .ok_or_else(|| ContractError::ClusterTokenNotSet {})?;

    // Use min as stale threshold if pro-rata redeem
    let stale_threshold = match asset_amounts {
        Some(_) => env.block.time.seconds() - FRESH_TIMESPAN,
        None => u64::MIN,
    };

    let cluster_state = query_cluster_state(
        deps.as_ref(),
        &env.contract.address.to_string(),
        stale_threshold,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let inv = cluster_state.inv;
    let target = cluster_state.target;

    let asset_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount.clone()).collect::<Vec<_>>();

    let asset_amounts: Vec<Uint128> = match &asset_amounts {
        Some(weights) => {
            let mut vec: Vec<Uint128> = vec![Uint128::zero(); asset_infos.len()];
            for i in 0..asset_infos.len() {
                for j in 0..weights.len() {
                    if weights[j].info == asset_infos[i] {
                        vec[i] = weights[j].amount;
                        break;
                    }
                }
            }
            vec
        }
        None => vec![],
    };

    let (collector_address, fee_rate) =
        query_collector_contract_address(&deps.querier, &cfg.factory)?;

    let fee_rate: FPDecimal = FPDecimal::from_str(&fee_rate)?;
    let keep_rate: FPDecimal = FPDecimal::one() - fee_rate;

    let _token_cap: u128 = (FPDecimal::from(max_tokens.u128()) * keep_rate).into();
    let token_cap: Uint128 = Uint128::from(_token_cap);

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

    let redeem_totals = redeem_response.redeem_assets;

    // check token_cost is exceeding max_tokens
    let _token_cost: FPDecimal = FPDecimal::from(redeem_response.token_cost.u128()) / keep_rate;
    let mut token_cost: u128 = _token_cost.into();
    if FPDecimal::from(token_cost) != _token_cost {
        token_cost += 1u128;
    }

    let token_cost: Uint128 = Uint128::from(token_cost);
    if token_cost > max_tokens {
        return Err(ContractError::AboveMaxTokens(token_cost, max_tokens));
    }

    // send redeem_totals to sender
    let mut messages: Vec<CosmosMsg> = redeem_totals
        .iter()
        .zip(asset_infos.iter())
        .filter(|(amt, _asset)| !amt.is_zero()) // remove 0 amounts
        .map(|(amt, asset_info)| {
            if let AssetInfo::Token { contract_addr, .. } = &asset_info {
                update_asset_balance(deps.storage, &contract_addr.to_string(), amt.clone(), false)?;
            } else if let AssetInfo::NativeToken { denom } = &asset_info {
                update_asset_balance(deps.storage, denom, amt.clone(), false)?;
            }
            let asset = Asset {
                info: asset_info.clone(),
                amount: amt.clone(),
            };

            match asset.into_msg(&deps.querier, sender.clone()) {
                Ok(msg) => Ok(msg),
                Err(e) => Err(ContractError::Std(e)),
            }
        })
        .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

    // compute fee_amt
    let _fee_amt: FPDecimal = FPDecimal::from(token_cost.u128()) * fee_rate;
    let mut fee_amt: u128 = _fee_amt.into();
    if FPDecimal::from(fee_amt) != _fee_amt {
        fee_amt += 1
    }

    // send fee to collector from allowance
    let fee_amt: Uint128 = Uint128::from(fee_amt);
    if !fee_amt.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: sender.to_string(),
                amount: fee_amt,
                recipient: collector_address.to_string(),
            })?,
            funds: vec![],
        }));
    }

    // burn the rest from allowance
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
            owner: sender.to_string(),
            amount: token_cost.checked_sub(fee_amt)?,
        })?,
        funds: vec![],
    }));

    // afterwards, notify the penalty contract that this update happened so
    // it can make stateful updates...
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

fn update_asset_balance(
    storage: &mut dyn Storage,
    asset_id: &String,
    amount: Uint128,
    mint: bool,
) -> Result<(), ContractError> {
    let mut asset_amount = match read_asset_balance(storage, &asset_id) {
        Ok(amount) => amount,
        Err(_) => Uint128::zero(),
    };

    match mint {
        true => asset_amount = asset_amount.checked_add(amount)?,
        false => asset_amount = asset_amount.checked_sub(amount)?,
    };

    store_asset_balance(storage, &asset_id, &asset_amount)?;
    Ok(())
}
