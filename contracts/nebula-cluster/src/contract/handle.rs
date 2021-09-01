#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128, WasmMsg,
};

use cw20::Cw20ExecuteMsg;

use crate::contract::{query_cluster_state, validate_targets};
use crate::error;
use crate::ext_query::{
    query_asset_balance, query_collector_contract_address, query_create_amount, query_redeem_amount,
};
use crate::state::{config_store, read_config};
use crate::state::{read_target_asset_data, save_target_asset_data};
use crate::util::vec_to_string;

use cluster_math::FPDecimal;
use nebula_protocol::cluster::ExecuteMsg;
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;

use std::str::FromStr;
use terraswap::asset::{Asset, AssetInfo};

// prices last 30s before they go from fresh to stale
const FRESH_TIMESPAN: u64 = 30;

/*
    Match the incoming message to the right category: receive, mint,
    reset_target, or  set cluster token
*/
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::RebalanceCreate {
            asset_amounts,
            min_tokens,
        } => create(deps, env, info, &asset_amounts, &min_tokens),
        ExecuteMsg::RebalanceRedeem {
            max_tokens,
            asset_amounts,
        } => receive_redeem(deps, env, info, max_tokens, asset_amounts),
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
) -> StdResult<Response> {
    // First, update cluster config
    config_store(deps.storage).update(|mut config| {
        if config.owner != info.sender.to_string() {
            return Err(StdError::generic_err(format!("unauthorized cluster update config {} {}", config.owner, info.sender.to_string())));
        }

        if let Some(owner) = owner {
            config.owner = owner;
        }

        if let Some(name) = name {
            config.name = name;
        }

        if let Some(description) = description {
            config.description = description;
        }

        match cluster_token {
            None => {}
            Some(_) => config.cluster_token = cluster_token,
        }

        if let Some(pricing_oracle) = pricing_oracle {
            config.pricing_oracle = pricing_oracle;
        }

        if let Some(target_oracle) = target_oracle {
            config.target_oracle = target_oracle;
        }

        if let Some(penalty) = penalty {
            config.penalty = penalty;
        }

        Ok(config)
    })?;

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
) -> StdResult<Response> {
    // allow removal / adding

    let cfg = read_config(deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(error::cluster_token_not_set());
    }
    // check permission
    if (info.sender.to_string() != cfg.owner) && (info.sender.to_string() != cfg.target_oracle) {
        return Err(StdError::generic_err("unauthorized update target"));
    }

    let mut asset_data = target.clone();

    // Create new vec for logging and validation purpose
    let mut updated_asset_infos = asset_data
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    let mut updated_target_weights = asset_data
        .iter()
        .map(|x| x.amount.clone())
        .collect::<Vec<_>>();

    if validate_targets(deps.querier, &env, updated_asset_infos.clone(), true).is_err() {
        return Err(StdError::generic_err(
            "Cluster must contain valid assets and cannot contain duplicate assets",
        ));
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
        let inv_balance = query_asset_balance(
            &deps.querier,
            &env.contract.address.to_string(),
            &prev_asset,
        )?;
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

    save_target_asset_data(deps.storage, &asset_data)?;

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
pub fn decommission(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    // allow removal / adding
    let cfg = read_config(deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(error::cluster_token_not_set());
    }
    // check permission for factory
    if info.sender.to_string() != cfg.factory {
        return Err(StdError::generic_err("unauthorized"));
    }

    // can only decommission an active cluster
    if !cfg.active {
        return Err(StdError::generic_err(
            "Cannot decommission an already decommissioned cluster",
        ));
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
    asset_amounts: &Vec<Asset>,
    min_tokens: &Option<Uint128>,
) -> StdResult<Response> {
    // duplication check for the given asset_amounts
    if validate_targets(
        deps.querier,
        &env,
        asset_amounts.iter().map(|a| a.info.clone()).collect(),
        true,
    )
    .is_err()
    {
        return Err(StdError::generic_err(
            "The given asset_amounts contain invalid or duplicate assets",
        ));
    }

    let cfg = read_config(deps.storage)?;

    if !cfg.active {
        return Err(StdError::generic_err(
            "Cannot call mint on a decommissioned cluster",
        ));
    }

    let cluster_state = query_cluster_state(
        deps.as_ref(),
        &env.contract.address.to_string(),
        env.block.time.seconds() - FRESH_TIMESPAN,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let mut inv = cluster_state.inv;
    let target = cluster_state.target;

    let asset_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let native_coin_denoms = asset_infos
        .iter()
        .filter(|asset| asset.is_native_token())
        .map(|asset| {
            match asset {
                AssetInfo::NativeToken { denom } => Ok(denom.clone()),
                _ => Err(StdError::generic_err(
                    "Already filtered. Cannot contain non-native denoms.",
                )),
            }
            .unwrap()
        })
        .collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount.clone()).collect::<Vec<_>>();

    let cluster_token = cfg
        .cluster_token
        .clone()
        .ok_or_else(|| error::cluster_token_not_set())?;

    // accommodate inputs: subsets of target assets vector
    let mut asset_weights = vec![Uint128::zero(); asset_infos.len()];
    let mut messages = vec![];

    // Return an error if assets not in target are sent to the mint function
    for coin in info.funds.iter() {
        if !native_coin_denoms.contains(&coin.denom) {
            return Err(StdError::generic_err(
                "Unsupported assets were sent to the mint function",
            ));
        }
    }

    for (i, asset_info) in asset_infos.iter().enumerate() {
        for asset in asset_amounts.iter() {
            if asset.info.clone() == asset_info.clone() {
                if target_weights[i] == Uint128::zero() && asset.amount > Uint128::zero() {
                    return Err(StdError::generic_err(
                        format!("Cannot mint with non-zero asset amount when target weight is zero for asset {}", asset.info.to_string()),
                    ));
                };

                asset_weights[i] = asset.amount;

                // pick up allowance from smart contracts
                if let AssetInfo::Token { contract_addr, .. } = &asset.info {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.clone(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount: asset.amount,
                        })?,
                        funds: vec![],
                    }));
                } else {
                    // validate that native token balance is correct
                    asset.assert_sent_native_token_balance(&info)?;

                    // inventory should not include native assets sent in this transaction
                    inv[i] = inv[i].checked_sub(asset.amount)?;
                }
                break;
            }
        }
    }

    let asset_weights = asset_weights.clone();

    let c = asset_weights;

    let mint_to_sender;

    // do a regular mint
    let mut extra_logs = vec![];
    if !cluster_token_supply.is_zero() {
        let mint_response = query_create_amount(
            &deps.querier,
            &cfg.penalty.clone(),
            env.block.height,
            cluster_token_supply,
            inv.clone(),
            c.clone(),
            prices.clone(),
            target_weights.clone(),
        )?;
        let mint_total = mint_response.create_tokens;

        let (collector_address, fee_rate) =
            query_collector_contract_address(&deps.querier, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&*fee_rate)?;

        // mint_to_sender = mint_total * (1 - fee_rate)
        // protocol_fee = mint_total - mint_to_sender == mint_total * fee_rate
        let _mint_to_sender: u128 =
            (FPDecimal::from(mint_total.u128()) * (FPDecimal::one() - fee_rate)).into();
        mint_to_sender = Uint128::from(_mint_to_sender);
        let protocol_fee = mint_total.checked_sub(mint_to_sender)?;

        // afterwards, notify the penalty contract that this update happened so
        // it can make stateful updates...
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.penalty.to_string(),
            msg: to_binary(&PenaltyExecuteMsg::PenaltyCreate {
                block_height: env.block.height,
                cluster_token_supply,
                inventory: inv,
                create_asset_amounts: c,
                asset_prices: prices,
                target_weights: target_weights,
            })?,
            funds: vec![],
        }));

        // actually mint the tokens
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

        extra_logs = mint_response.attributes;
        extra_logs.push(attr("fee_amt", protocol_fee))
    } else {
        // cluster has no cluster tokens -- cluster is empty and needs to be initialized
        // attempt to initialize it with min_tokens as the number of cluster tokens
        // and the mint cluster c as the initial assets
        // c is required to be in ratio with the target weights
        if let Some(proposed_mint_total) = min_tokens {
            let mut val = 0;
            for i in 0..c.len() {
                if c[i].u128() % target_weights[i].u128() != 0 {
                    return Err(StdError::generic_err(format!(
                        "Initial cluster assets must be a multiple of target weights at index {}",
                        i
                    )));
                }

                let div = target_weights[i].u128() / c[i].u128();
                if val == 0 {
                    val = div;
                }

                if div != val {
                    return Err(StdError::generic_err(format!(
                        "Initial cluster assets must be a multiple of target weights at index {}",
                        i
                    )));
                }
            }

            mint_to_sender = *proposed_mint_total;
        } else {
            return Err(StdError::generic_err(
                "Cluster is uninitialized. \
            To initialize it with your mint cluster, \
            provide min_tokens as the amount of cluster tokens you want to start with.",
            ));
        }
    }

    if let Some(min) = min_tokens {
        if mint_to_sender < *min {
            return Err(error::below_min_tokens(mint_to_sender, *min));
        }
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            amount: mint_to_sender,
            recipient: info.sender.to_string(),
        })?,
        funds: vec![],
    }));

    let mut logs = vec![
        attr("action", "mint"),
        attr("sender", &info.sender.to_string()),
        attr("mint_to_sender", mint_to_sender),
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
) -> StdResult<Response> {
    let sender = info.sender.to_string();

    let cfg = read_config(deps.storage)?;

    // If cluster is not active, must do pro rata redeem
    if !cfg.active && asset_amounts.is_some() {
        return Err(StdError::generic_err(
            "Cannot call non pro-rata redeem on a decommissioned cluster",
        ));
    }

    let asset_amounts = if !cfg.active { None } else { asset_amounts };

    let cluster_token = cfg
        .cluster_token
        .ok_or_else(|| error::cluster_token_not_set())?;

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
        return Err(error::above_max_tokens(token_cost, max_tokens));
    }

    // send redeem_totals to sender
    let mut messages: Vec<CosmosMsg> = redeem_totals
        .iter()
        .zip(asset_infos.iter())
        .filter(|(amt, _asset)| !amt.is_zero()) // remove 0 amounts
        .map(|(amt, asset_info)| {
            let asset = Asset {
                info: asset_info.clone(),
                amount: amt.clone(),
            };

            // TODO: Check if sender field is correct here (recipient should be sender.clone())
            asset.into_msg(&deps.querier, Addr::unchecked(sender.clone()))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

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
                owner: sender.clone(),
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
            owner: sender.clone(),
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
            target_weights: target_weights,
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
