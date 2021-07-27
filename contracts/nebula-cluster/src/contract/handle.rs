use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::Cw20HandleMsg;

use crate::contract::{query_cluster_state, validate_targets};
use crate::error;
use crate::ext_query::{
    query_asset_balance, query_collector_contract_address, query_mint_amount,
    query_redeem_amount,
};
use crate::state::{config_store, read_config};
use crate::state::{read_target_asset_data, save_target_asset_data};
use crate::util::vec_to_string;

use cluster_math::FPDecimal;
use nebula_protocol::cluster::HandleMsg;
use nebula_protocol::penalty::{HandleMsg as PenaltyHandleMsg, QueryMsg as PenaltyQueryMsg};
 
use std::str::FromStr;
use std::u32;
use terraswap::asset::{Asset, AssetInfo};

// prices last 30s before they go from fresh to stale
const FRESH_TIMESPAN: u64 = 30;

/*
    Match the incoming message to the right category: receive, mint,
    reset_target, or  set cluster token
*/
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Mint {
            asset_amounts,
            min_tokens,
        } => mint(deps, env, &asset_amounts, &min_tokens),
        HandleMsg::Burn {
            max_tokens,
            asset_amounts,
        } => receive_burn(deps, env, max_tokens, asset_amounts),
        HandleMsg::UpdateConfig {
            owner,
            name,
            description,
            cluster_token,
            pricing_oracle,
            composition_oracle,
            penalty,
            target,
        } => update_config(
            deps,
            env,
            owner,
            name,
            description,
            cluster_token,
            pricing_oracle,
            composition_oracle,
            penalty,
            target,
        ),
        HandleMsg::UpdateTarget { target } => update_target(deps, env, &target),
        HandleMsg::RevokeAsset {} => revoke_asset(deps, env),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    name: Option<String>,
    description: Option<String>,
    cluster_token: Option<HumanAddr>,
    pricing_oracle: Option<HumanAddr>,
    composition_oracle: Option<HumanAddr>,
    penalty: Option<HumanAddr>,
    target: Option<Vec<Asset>>,
) -> HandleResult {
    // First, update cluster config
    config_store(&mut deps.storage).update(|mut config| {
        if config.owner != env.message.sender {
            return Err(StdError::unauthorized());
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

        if let Some(composition_oracle) = composition_oracle {
            config.composition_oracle = composition_oracle;
        }

        if let Some(penalty) = penalty {
            config.penalty = penalty;
        }

        Ok(config)
    })?;

    match target {
        None => HandleResponse::default(),
        Some(target) => update_target(deps, env, &target)?,
    };

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

/*
    Changes the cluster target weights for different assets to the given
    target weights and saves it. The ordering of the target weights is
    determined by the given assets.
*/
pub fn update_target<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    target: &Vec<Asset>,
) -> StdResult<HandleResponse> {
    // allow removal / adding

    let cfg = read_config(&deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(error::cluster_token_not_set());
    }
    // check permission
    if (env.message.sender != cfg.owner) && (env.message.sender != cfg.composition_oracle) {
        return Err(StdError::unauthorized());
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

    if validate_targets(&deps, &env, updated_asset_infos.clone(), None).is_err() {
        return Err(StdError::generic_err(
            "Cluster must contain valid assets and cannot contain duplicate assets",
        ));
    }

    // Load previous assets & target
    let (prev_assets, prev_target): (Vec<AssetInfo>, Vec<Uint128>) =
        read_target_asset_data(&deps.storage)?
            .iter()
            .map(|x| (x.info.clone(), x.amount.clone()))
            .unzip();

    // When previous assets are not found,
    // then set that not found item target to zero
    for prev_asset in prev_assets.iter() {
        let inv_balance = query_asset_balance(&deps.querier, &env.contract.address, &prev_asset)?;
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

    save_target_asset_data(&mut deps.storage, &asset_data)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "reset_target"),
            log("prev_assets", vec_to_string(&prev_assets)),
            log("prev_targets", vec_to_string(&prev_target)),
            log("updated_assets", vec_to_string(&updated_asset_infos)),
            log("updated_targets", vec_to_string(&updated_target_weights)),
        ],
        data: None,
    })
}

/*

*/
pub fn revoke_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // allow removal / adding

    let cfg = read_config(&deps.storage)?;
    if let None = cfg.cluster_token {
        return Err(error::cluster_token_not_set());
    }
    // check permission for factory
    if env.message.sender != cfg.factory {
        return Err(StdError::unauthorized());
    }

    // can only revoke an active cluster
    if !cfg.active {
        return Err(StdError::unauthorized());
    }

    config_store(&mut deps.storage).update(|mut config| {
        config.active = false;

        Ok(config)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "revoke_asset")],
        data: None,
    })
}

/*
    Mint cluster tokens from the asset amounts given.
    Throws error if there can only be less than 'min_tokens' minted from the assets.
*/
pub fn mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_amounts: &Vec<Asset>,
    min_tokens: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    // duplication check for the given asset_amounts
    if validate_targets(&deps, &env, asset_amounts.iter().map(|a| a.info.clone()).collect(), None).is_err() {
        return Err(StdError::generic_err(
            "The given asset_amounts contain invalid or duplicate assets",
        ));
    }

    let cluster_state = query_cluster_state(
        &deps,
        &env.contract.address,
        env.block.time - FRESH_TIMESPAN,
    )?;

    if !cluster_state.active {
        return Err(StdError::generic_err(
            "Trying to mint on a deactivated cluster",
        ));
    }

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let mut inv = cluster_state.inv;
    let target = cluster_state.target;

    let cfg = read_config(&deps.storage)?;

    let asset_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount.clone()).collect::<Vec<_>>();

    let cluster_token = cfg
        .cluster_token
        .clone()
        .ok_or_else(|| error::cluster_token_not_set())?;

    // accommodate inputs: subsets of target assets vector
    let mut asset_weights = vec![Uint128::zero(); asset_infos.len()];
    let mut messages = vec![];

    // Return an error if assets not in target are sent to the mint function
    for asset in asset_amounts.iter() {
        if !asset_infos.contains(&asset.info) {
            return Err(StdError::generic_err(
                "Unsupported assets were sent to the mint function",
            ));
        }
    }

    for (i, asset_info) in asset_infos.iter().enumerate() {
        for asset in asset_amounts.iter() {
            if asset.info.clone() == asset_info.clone() {
                asset_weights[i] = asset.amount;

                // pick up allowance from smart contracts
                if let AssetInfo::Token { contract_addr, .. } = &asset.info {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.clone(),
                        msg: to_binary(&Cw20HandleMsg::TransferFrom {
                            owner: env.message.sender.clone(),
                            recipient: env.contract.address.clone(),
                            amount: asset.amount,
                        })?,
                        send: vec![],
                    }));
                } else {
                    // validate that native token balance is correct
                    asset.assert_sent_native_token_balance(&env)?;

                    // inventory should not include native assets sent in this transaction
                    inv[i] = (inv[i] - asset.amount)?;
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
        let mint_response = query_mint_amount(
            &deps.querier,
            &cfg.penalty.clone(),
            env.block.height,
            cluster_token_supply,
            inv.clone(),
            c.clone(),
            prices.clone(),
            target_weights.clone(),
        )?;
        let mint_total = mint_response.mint_tokens;

        let (collector_address, fee_rate) =
            query_collector_contract_address(&deps.querier, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&*fee_rate)?;

        // mint_to_sender = mint_total * (1 - fee_rate)
        // protocol_fee = mint_total - mint_to_sender == mint_total * fee_rate
        let _mint_to_sender: u128 =
            (FPDecimal::from(mint_total.u128()) * (FPDecimal::one() - fee_rate)).into();
        mint_to_sender = Uint128::from(_mint_to_sender);
        let protocol_fee = (mint_total - mint_to_sender)?;

        // afterwards, notify the penalty contract that this update happened so
        // it can make stateful updates...
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.penalty.clone(),
            msg: to_binary(&PenaltyHandleMsg::Mint {
                block_height: env.block.height,
                cluster_token_supply,
                inventory: inv,
                mint_asset_amounts: c,
                asset_prices: prices,
                target_weights: target_weights,
            })?,
            send: vec![],
        }));

        // actually mint the tokens
        if !protocol_fee.is_zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster_token.clone(),
                msg: to_binary(&Cw20HandleMsg::Mint {
                    amount: protocol_fee,
                    recipient: collector_address.clone(),
                })?,
                send: vec![],
            }));
        }

        extra_logs = mint_response.log;
        extra_logs.push(log("fee_amt", protocol_fee))
    } else {
        // cluster has no cluster tokens -- cluster is empty and needs to be initialized
        // attempt to initialize it with min_tokens as the number of cluster tokens
        // and the mint cluster c as the initial assets
        // c is required to be in ratio with the target weights
        if let Some(proposed_mint_total) = min_tokens {
            let mut val = 0;
            for i in 0..c.len() {
                // if target[i] % (c[i].u128() as u32) != 0u32 {
                //     return Err(StdError::generic_err(format!(
                //         "1. Initial cluster assets must be in target weights {} {} {}",
                //         target[i] % (c[i].u128() as u32),
                //         target[i],
                //         c[i].u128() as u32
                //     )));
                // }
                let div = target_weights[i].u128() / c[i].u128();
                if val == 0 {
                    val = div;
                }

                if div != val {
                    return Err(StdError::generic_err(format!(
                        "Initial cluster assets must be in target weights {} {} {} {}",
                        div,
                        val,
                        target[i],
                        c[i].u128() as u32
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
        contract_addr: cluster_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Mint {
            amount: mint_to_sender,
            recipient: env.message.sender.clone(),
        })?,
        send: vec![],
    }));

    let mut logs = vec![
        log("action", "mint"),
        log("sender", &env.message.sender),
        log("mint_to_sender", mint_to_sender),
    ];
    logs.extend(extra_logs);

    // mint and send number of tokens to user
    Ok(HandleResponse {
        messages,
        log: logs,
        data: None,
    })
}

/*
    Receives cluster tokens which are burned for assets according to
    the given asset_weights and cluster penalty parameter. The corresponding
    assets are taken from the cluster inventory and sent back to the user
    along with any rewards based on whether the assets are moved towards/away
    from the target.
*/
pub fn receive_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    let sender = env.message.sender.clone();

    let cfg = read_config(&deps.storage)?;

    // Is there an idiomatic way to do this?
    let asset_amounts = if cfg.active { None } else { asset_amounts };

    let cluster_token = cfg
        .cluster_token
        .ok_or_else(|| error::cluster_token_not_set())?;

    let cluster_state = query_cluster_state(
        &deps,
        &env.contract.address,
        env.block.time - FRESH_TIMESPAN,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let inv = cluster_state.inv;
    let target = cluster_state.target;

    let asset_infos = target.iter().map(|x| x.info.clone()).collect::<Vec<_>>();

    let target_weights = target.iter().map(|x| x.amount.clone()).collect::<Vec<_>>();

    let asset_amounts: Vec<Uint128> = match &asset_amounts {
        Some(weights) => {
            let mut vec: Vec<Uint128> = vec![Uint128(0); asset_infos.len()];
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
            asset.into_msg(&deps, env.contract.address.clone(), sender.clone())
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
            contract_addr: cluster_token.clone(),
            msg: to_binary(&Cw20HandleMsg::TransferFrom {
                owner: sender.clone(),
                amount: fee_amt,
                recipient: collector_address.clone(),
            })?,
            send: vec![],
        }));
    }

    // burn the rest from allowance
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.clone(),
        msg: to_binary(&Cw20HandleMsg::BurnFrom {
            owner: sender.clone(),
            amount: (token_cost - fee_amt)?,
        })?,
        send: vec![],
    }));

    // afterwards, notify the penalty contract that this update happened so
    // it can make stateful updates...
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.penalty.clone(),
        msg: to_binary(&PenaltyQueryMsg::Redeem {
            block_height: env.block.height,
            cluster_token_supply,
            inventory: inv,
            max_tokens,
            redeem_asset_amounts: asset_amounts.clone(),
            asset_prices: prices,
            target_weights: target_weights,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        log: vec![
            vec![
                log("action", "receive:burn"),
                log("sender", sender),
                log("burn_amount", (token_cost - fee_amt)?),
                log("token_cost", token_cost),
                log("kept_as_fee", fee_amt),
                log("asset_amounts", vec_to_string(&asset_amounts)),
                log("redeem_totals", vec_to_string(&redeem_totals)),
            ],
            redeem_response.log,
        ]
        .concat(),
        data: None,
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_helper::*;
    use nebula_protocol::cluster::HandleMsg;
    use pretty_assertions::assert_eq;
    use terraswap::asset::{Asset, AssetInfo};

    #[test]
    fn mint() {
        let (mut deps, _) = mock_init();
        mock_querier_setup(&mut deps);
        // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
        // ---
        // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
        // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
        // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
        // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389

        // The set token balance should include the amount we would also like to stage
        deps.querier
            .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
            .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
            .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
            .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
            .set_oracle_prices(vec![
                ("mAAPL", Decimal::from_str("135.18").unwrap()),
                ("mGOOG", Decimal::from_str("1780.03").unwrap()),
                ("mMSFT", Decimal::from_str("222.42").unwrap()),
                ("mNFLX", Decimal::from_str("540.82").unwrap()),
            ]);

        let asset_amounts = vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128(125_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::zero(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128(149_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128(50_090_272),
            },
        ];

        deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

        let mint_msg = HandleMsg::Mint {
            asset_amounts: asset_amounts.clone(),
            min_tokens: None,
        };

        let addr = "addr0000";
        let env = mock_env(h(addr), &[]);
        let res = handle(&mut deps, env, mint_msg).unwrap();

        assert_eq!(5, res.log.len());

        for log in res.log.iter() {
            match log.key.as_str() {
                "action" => assert_eq!("mint", log.value),
                "sender" => assert_eq!(addr, log.value),
                "mint_to_sender" => assert_eq!("98", log.value),
                "penalty" => assert_eq!("1234", log.value),
                "fee_amt" => assert_eq!("1", log.value),
                &_ => panic!("Invalid value found in log")
            }
        }

        assert_eq!(7, res.messages.len());
    }

    
    #[test]
    fn burn() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);
    
        deps.querier
            .set_token_supply(consts::cluster_token(), 100_000_000)
            .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000)
            .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
            .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
            .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
            .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
            .set_oracle_prices(vec![
                ("mAAPL", Decimal::from_str("135.18").unwrap()),
                ("mGOOG", Decimal::from_str("1780.03").unwrap()),
                ("mMSFT", Decimal::from_str("222.42").unwrap()),
                ("mNFLX", Decimal::from_str("540.82").unwrap()),
            ]);

        let addr = "addr0000";
    
        let msg = HandleMsg::Burn {
            max_tokens: Uint128(20_000_000),
            asset_amounts: None,
        };
        let env = mock_env(h(addr), &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        
        assert_eq!(8, res.log.len());
        
        for log in res.log.iter() {
            match log.key.as_str() {
                "action" => assert_eq!("receive:burn", log.value),
                "sender" => assert_eq!(addr, log.value),
                "burn_amount" => assert_eq!("1234", log.value),
                "token_cost" => assert_eq!("1247", log.value),
                "kept_as_fee" => assert_eq!("13", log.value),
                "asset_amounts" => assert_eq!("[]", log.value),
                "redeem_totals" => assert_eq!("[99, 98, 97, 96]", log.value),
                "penalty" => assert_eq!("1234", log.value),
                &_ => panic!("Invalid value found in log")
            }
        }

        assert_eq!(7, res.messages.len());
    }

    #[test]
    fn update_target() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        deps.querier
            .set_token_supply(consts::cluster_token(), 100_000_000)
            .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

        let new_target: Vec<Asset> = vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128(10),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128(5),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128(35),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGME"),
                },
                amount: Uint128(50),
            },
        ];
        let msg = HandleMsg::UpdateTarget { target: new_target };

        let env = mock_env(consts::owner(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();

        assert_eq!(5, res.log.len());
        
        for log in res.log.iter() {
            match log.key.as_str() {
                "action" => assert_eq!("reset_target", log.value),
                "prev_assets" => assert_eq!("[mAAPL, mGOOG, mMSFT, mNFLX]", log.value),
                "prev_targets" => assert_eq!("[20, 20, 20, 20]", log.value),
                "updated_assets" => assert_eq!("[mAAPL, mGOOG, mMSFT, mGME, mNFLX]", log.value),
                "updated_targets" => assert_eq!("[10, 5, 35, 50, 0]", log.value),
                &_ => panic!("Invalid value found in log")
            }
        }
        assert_eq!(0, res.messages.len());
    }
}
