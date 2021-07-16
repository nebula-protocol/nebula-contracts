use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage, Uint128, WasmMsg,
};

use cw20::Cw20HandleMsg;
use error::bad_weight_values;

use crate::contract::{query_cluster_state, validate_targets};
use crate::error;
use crate::ext_query::{
    query_collector_contract_address, query_cw20_balance, query_mint_amount, query_redeem_amount,
    ExtQueryMsg,
};
use crate::state::{read_config, save_config, TargetAssetData};
use crate::state::{read_target_asset_data, save_target_asset_data};
use crate::util::vec_to_string;
use cluster_math::FPDecimal;
use nebula_protocol::cluster::HandleMsg;
use std::str::FromStr;
use std::u32;
use terraswap::asset::{Asset, AssetInfo};
use terraswap::querier::query_balance;

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
        } => try_mint(deps, env, &asset_amounts, &min_tokens),
        HandleMsg::Burn {
            max_tokens,
            asset_amounts,
        } => try_receive_burn(deps, env, max_tokens, asset_amounts),
        HandleMsg::ResetTarget { assets, target } => try_reset_target(deps, env, &assets, &target),
        HandleMsg::_SetClusterToken { cluster_token } => {
            try_set_cluster_token(deps, env, &cluster_token)
        }
        HandleMsg::ResetCompositionOracle { composition_oracle } => {
            try_reset_composition_oracle(deps, env, &composition_oracle)
        }
        HandleMsg::ResetPenalty { penalty } => try_reset_penalty(deps, env, &penalty),
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, env, &owner),
    }
}

/*
    Receives cluster tokens which are burned for assets according to
    the given asset_weights and cluster penalty paramter. The corresponding
    assets are taken from the cluster inventory and sent back to the user
    along with any rewards based on whether the assets are moved towards/away
    from the target.
*/
pub fn try_receive_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    max_tokens: Uint128,
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    let sender = env.message.sender.clone();

    let cfg = read_config(&deps.storage)?;
    let cluster_token = cfg
        .cluster_token
        .clone()
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

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let asset_infos = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let asset_amounts: Vec<Uint128> = match &asset_amounts {
        Some(weights) => {
            let mut vec: Vec<Uint128> = vec![Uint128(0); asset_infos.len()];
            for i in 0..asset_infos.len() {
                for j in 0..weights.len() {
                    if weights[j].info.clone() == asset_infos[i].clone() {
                        vec[i] = weights[j].amount;
                        break;
                    }
                }
            }
            vec
        }
        None => vec![],
    };

    let (collector_address, fee_rate) = query_collector_contract_address(&deps, &cfg.factory)?;
    let fee_rate: FPDecimal = FPDecimal::from_str(&*fee_rate)?;
    let keep_rate: FPDecimal = FPDecimal::one() - fee_rate;

    let token_cap = Uint128((FPDecimal::from(max_tokens.u128()) * keep_rate).into());

    let redeem_response = query_redeem_amount(
        &deps,
        &cfg.penalty,
        env.block.height,
        cluster_token_supply,
        inv.clone(),
        token_cap,
        asset_amounts.clone(),
        prices.clone(),
        target.clone(),
    )?;

    let redeem_totals = redeem_response.redeem_assets;

    let estimated_cst = FPDecimal::from(redeem_response.token_cost.u128()) / keep_rate;
    let mut token_cost = estimated_cst.into();

    if FPDecimal::from(token_cost) != estimated_cst {
        token_cost += 1;
    }

    let token_cost = Uint128(token_cost);

    if token_cost > max_tokens {
        return Err(error::above_max_tokens(token_cost, max_tokens));
    }

    let _fee_amt = FPDecimal::from(token_cost.u128()) * fee_rate;

    let mut fee_amt = _fee_amt.into();
    if FPDecimal::from(fee_amt) != _fee_amt {
        fee_amt += 1
    }

    let fee_amt = Uint128(fee_amt);

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

    // extract cluster tokens from allowance
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.clone(),
        msg: to_binary(&Cw20HandleMsg::TransferFrom {
            owner: sender.clone(),
            recipient: env.contract.address.clone(),
            amount: token_cost,
        })?,
        send: vec![],
    }));

    // send fee to collector
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Transfer {
            amount: fee_amt,
            recipient: collector_address.clone(),
        })?,
        send: vec![],
    }));

    // burn the rest
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cluster_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Burn {
            amount: (token_cost - fee_amt)?,
        })?,
        send: vec![],
    }));

    // afterwards, notify the penalty contract that this update happened so
    // it can make stateful updates...
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.penalty.clone(),
        msg: to_binary(&ExtQueryMsg::Redeem {
            block_height: env.block.height,
            cluster_token_supply,
            inventory: inv,
            max_tokens,
            redeem_asset_amounts: asset_amounts.clone(),
            asset_prices: prices,
            target_weights: target,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        log: vec![
            vec![
                log("action", "receive:burn"),
                log("sender", sender.clone()),
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

/*
    Changes the cluster target weights for different assets to the given
    target weights and saves it. The ordering of the target weights is
    determined by the given assets.
*/
pub fn try_reset_target<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    assets: &Vec<AssetInfo>,
    target: &[u32],
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

    // //TODO: Make sure all assets in new asset vector actually exist
    // let provided: u32 = target.clone().iter().sum();
    // if provided != 100 {
    //     return Err(bad_weight_values(provided));
    // }

    if target.len() != assets.len() {
        return Err(error::bad_weight_dimensions(target.len(), assets.len()));
    }

    if !validate_targets(assets.clone()) {
        return Err(StdError::generic_err(
            "Cluster cannot contain duplicate assets",
        ));
    }

    let mut asset_data: Vec<TargetAssetData> = Vec::new();
    for i in 0..target.len() {
        let asset_elem = TargetAssetData {
            asset: assets[i].clone(),
            target: target[i].clone(),
        };
        asset_data.push(asset_elem);
    }

    let updated_assets = asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let prev_asset_data = read_target_asset_data(&deps.storage)?;
    let prev_assets = prev_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let prev_target = prev_asset_data.iter().map(|x| x.target).collect::<Vec<_>>();

    for i in 0..prev_assets.len() {
        let prev_asset = &prev_assets[i];
        let inv_balance = match prev_asset {
            AssetInfo::Token { contract_addr } => {
                query_cw20_balance(&deps, &contract_addr, &env.contract.address)
            }
            AssetInfo::NativeToken { denom } => {
                query_balance(&deps, &env.contract.address, denom.clone())
            }
        };

        if !inv_balance?.is_zero() && !updated_assets.contains(&prev_asset) {
            let asset_elem = TargetAssetData {
                asset: prev_asset.clone(),
                target: 0,
            };
            asset_data.push(asset_elem);
        }
    }

    save_target_asset_data(&mut deps.storage, &asset_data)?;

    let updated_assets = asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let updated_target = asset_data.iter().map(|x| x.target).collect::<Vec<_>>();

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "reset_target"),
            log("prev_assets", vec_to_string(&prev_assets)),
            log("prev_targets", vec_to_string(&prev_target)),
            log("updated_assets", vec_to_string(&updated_assets)),
            log("updated_targets", vec_to_string(&updated_target)),
        ],
        data: None,
    })
}

// Changes the composotion oracle contract for this cluster
pub fn try_reset_composition_oracle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    composition_oracle: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let mut new_cfg = cfg.clone();
    new_cfg.composition_oracle = composition_oracle.clone();
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "reset_composition_oracle"),
            log("composition_oracle", &composition_oracle),
        ],
        data: None,
    })
}

// Changes the penalty contract for this cluster
pub fn try_reset_penalty<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    penalty: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let mut new_cfg = cfg.clone();
    new_cfg.penalty = penalty.clone();
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "reset_penalty"), log("penalty", &penalty)],
        data: None,
    })
}

/*
     May be called by the Cluster contract owner to set the cluster token for first time
*/
pub fn try_set_cluster_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cluster_token: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    // check if already set
    if let Some(token) = cfg.cluster_token {
        return Err(error::cluster_token_already_set(&token));
    }

    let mut new_cfg = cfg.clone();
    new_cfg.cluster_token = Some(cluster_token.clone());
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_set_cluster_token"),
            log("cluster_token", &cluster_token),
        ],
        data: None,
    })
}

/*
     May be called by the Cluster contract owner to reset the owner
*/
pub fn try_reset_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    // TODO: Error checking needed here? can this function be called more than once?
    // if let Some(token) = cfg.cluster_token {
    //     return Err(error::cluster_token_already_set(&token));
    // }

    let mut new_cfg = cfg.clone();
    new_cfg.owner = owner.clone();
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_try_reset_owner"),
            log("cluster_token", &owner),
        ],
        data: None,
    })
}

/*
    Tries to mint cluster tokens from the asset amounts given.
    Throws error if there can only be less than 'min_tokens' minted from the assets.
*/
pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_amounts: &Vec<Asset>,
    min_tokens: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    let cluster_state = query_cluster_state(
        &deps,
        &env.contract.address,
        env.block.time - FRESH_TIMESPAN,
    )?;

    let prices = cluster_state.prices;
    let cluster_token_supply = cluster_state.outstanding_balance_tokens;
    let mut inv = cluster_state.inv;
    let target = cluster_state.target;

    let cfg = read_config(&deps.storage)?;

    let target_asset_data = read_target_asset_data(&deps.storage)?;

    let asset_infos = &target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let cluster_token = cfg
        .cluster_token
        .clone()
        .ok_or_else(|| error::cluster_token_not_set())?;

    // accommmodate inputs: subsets of target assets vector
    let mut asset_weights = vec![Uint128(0); asset_infos.len()];
    let mut messages = vec![];

    //Return an error if native assets not in target are sent to the mint function
    for asset in asset_amounts.iter() {
        if !asset_infos.contains(&asset.info) {
            return Err(StdError::generic_err(
                "Unsupported native assets were sent to the mint function",
            ));
        }
    }

    for i in 0..asset_infos.len() {
        for asset in asset_amounts.iter() {
            if asset.info.clone() == asset_infos[i].clone() {
                asset_weights[i] = asset.amount;

                // validate that native token balance is correct
                asset.assert_sent_native_token_balance(&env)?;
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

    let mut extra_logs = vec![];
    // do a regular mint
    if cluster_token_supply != Uint128::zero() {
        let mint_response = query_mint_amount(
            &deps,
            &cfg.penalty.clone(),
            env.block.height,
            cluster_token_supply,
            inv.clone(),
            c.clone(),
            prices.clone(),
            target.clone(),
        )?;
        let mint_total = mint_response.mint_tokens;

        let (collector_address, fee_rate) = query_collector_contract_address(&deps, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&*fee_rate)?;

        // Decimal doesn't give the ability to subtract...
        mint_to_sender =
            Uint128((FPDecimal::from(mint_total.u128()) * (FPDecimal::one() - fee_rate)).into());
        let protocol_fee = (mint_total - mint_to_sender)?;

        // afterwards, notify the penalty contract that this update happened so
        // it can make stateful updates...
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.penalty.clone(),
            msg: to_binary(&ExtQueryMsg::Mint {
                block_height: env.block.height,
                cluster_token_supply,
                inventory: inv,
                mint_asset_amounts: c,
                asset_prices: prices,
                target_weights: target,
            })?,
            send: vec![],
        }));

        // actually mint the tokens
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Mint {
                amount: protocol_fee,
                recipient: collector_address.clone(),
            })?,
            send: vec![],
        }));

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
                if target[i] % c[i].u128() as u32 != 0u32 {
                    return Err(StdError::generic_err(
                        "Initial cluster assets must be in target weights",
                    ));
                }
                let div = target[i] / c[i].u128() as u32;
                if val == 0 {
                    val = div;
                }
                if div != val {
                    return Err(StdError::generic_err(
                        "Initial cluster assets must be in target weights",
                    ));
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

#[cfg(test)]
mod tests {

    use crate::test_helper::*;
    use nebula_protocol::cluster::HandleMsg;
    use pretty_assertions::assert_eq;
    use terraswap::asset::{Asset, AssetInfo};

    #[test]
    fn mint() {
        let (mut deps, _init_res) = mock_init();

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
        let mint_msg = HandleMsg::Mint {
            asset_amounts: asset_amounts.clone(),
            min_tokens: None,
        };

        let env = mock_env(h("addr0000"), &[]);
        // let res = handle(&mut deps, env, mint_msg).unwrap();

        // match res {
        //     Err(..) => (),
        //     _ => panic!("requires staging"),
        // }

        // let env = mock_env(h("addr0000"), &[]);
        // let res = handle(&mut deps, env, mint_msg).unwrap();

        // for log in res.log.iter() {
        //     println!("{}: {}", log.key, log.value);
        // }
        // assert_eq!(1, res.messages.len());
    }
    //
    // #[test]
    // // Should be same output as mint()
    // fn mint_two() {
    //     let (mut deps, _init_res) = mock_init();
    //     mock_querier_setup(&mut deps);
    //
    //     // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
    //     // ---
    //     // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
    //     // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
    //     // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
    //     // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389
    //     deps.querier
    //         .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
    //         .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
    //         .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
    //         .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
    //         .set_oracle_prices(vec![
    //             ("mAAPL", Decimal::from_str("135.18").unwrap()),
    //             ("mGOOG", Decimal::from_str("1780.03").unwrap()),
    //             ("mMSFT", Decimal::from_str("222.42").unwrap()),
    //             ("mNFLX", Decimal::from_str("540.82").unwrap()),
    //         ]);
    //
    //     let asset_amounts = vec![
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mMSFT"),
    //             },
    //             amount: Uint128(149_000_000),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mNFLX"),
    //             },
    //             amount: Uint128(50_090_272),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mAAPL"),
    //             },
    //             amount: Uint128(125_000_000),
    //         },
    //     ];
    //     let mint_msg = HandleMsg::Mint {
    //         asset_amounts: asset_amounts.clone(),
    //         min_tokens: None,
    //     };
    //
    //     let env = mock_env(h("addr0000"), &[]);
    //     let res = handle(&mut deps, env, mint_msg.clone());
    //     match res {
    //         Err(..) => (),
    //         _ => panic!("requires staging"),
    //     }
    //
    //     for asset in asset_amounts {
    //         let env = mock_env(
    //             match asset.info {
    //                 AssetInfo::Token { contract_addr } => contract_addr,
    //                 AssetInfo::NativeToken { denom } => h(&denom),
    //             },
    //             &[],
    //         );
    //         let stage_asset_msg = HandleMsg::Receive(Cw20ReceiveMsg {
    //             sender: h("addr0000"),
    //             msg: Some(to_binary(&Cw20HookMsg::StageAsset {}).unwrap()),
    //             amount: asset.amount,
    //         });
    //         handle(&mut deps, env, stage_asset_msg).unwrap();
    //     }
    //
    //     let env = mock_env(h("addr0000"), &[]);
    //     let res = handle(&mut deps, env, mint_msg).unwrap();
    //
    //     for log in res.log.iter() {
    //         println!("{}: {}", log.key, log.value);
    //     }
    //     assert_eq!(1, res.messages.len());
    // }
    //
    // #[test]
    // fn mint_with_native_stage() {
    //     let (mut deps, _init_res) = mock_init_native_stage();
    //     mock_querier_setup_stage_native(&mut deps);
    //
    //     deps.querier
    //         .set_token_balance("wBTC", consts::cluster_token(), 1_000_000)
    //         .set_denom_balance("uluna", MOCK_CONTRACT_ADDR, 2_000_000)
    //         .set_oracle_prices(vec![
    //             ("wBTC", Decimal::from_str("30000.00").unwrap()),
    //             ("uluna", Decimal::from_str("15.00").unwrap()),
    //         ]);
    //
    //     let asset_amounts = vec![
    //         Asset {
    //             info: AssetInfo::NativeToken {
    //                 denom: "uluna".to_string(),
    //             },
    //             amount: Uint128(1_000_000),
    //         },
    //     ];
    //
    //     let mint_msg = HandleMsg::Mint {
    //         asset_amounts: asset_amounts.clone(),
    //         min_tokens: None,
    //     };
    //
    //     let env = mock_env(h("addr0000"), &[]);
    //     let res = handle(&mut deps, env, mint_msg.clone());
    //     match res {
    //         Err(..) => (),
    //         _ => panic!("requires staging"),
    //     }
    //
    //     for asset in asset_amounts {
    //         let env = mock_env(
    //             h("addr0000"),
    //             &[Coin {
    //                 denom: "uluna".to_string(),
    //                 amount: Uint128(1_000_000),
    //             }],
    //         );
    //
    //         if asset.is_native_token() {
    //             let stage_asset_msg = HandleMsg::StageNativeAsset { asset };
    //             handle(&mut deps, env, stage_asset_msg).unwrap();
    //         };
    //
    //     }
    //
    //     let env = mock_env(h("addr0000"), &[]);
    //     let res = handle(&mut deps, env, mint_msg).unwrap();
    //
    //     for log in res.log.iter() {
    //         println!("{}: {}", log.key, log.value);
    //     }
    //     assert_eq!(1, res.messages.len());
    // }
    //
    // #[test]
    // fn burn() {
    //     let (mut deps, _init_res) = mock_init();
    //     mock_querier_setup(&mut deps);
    //
    //     deps.querier
    //         .set_token_supply(consts::cluster_token(), 100_000_000)
    //         .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);
    //
    //     let new_assets = vec![
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mAAPL"),
    //             },
    //             amount: Uint128(10),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mGOOG"),
    //             },
    //             amount: Uint128(10),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mMSFT"),
    //             },
    //             amount: Uint128(10),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mNFLX"),
    //             },
    //             amount: Uint128(10),
    //         },
    //     ];
    //
    //     let msg = HandleMsg::Receive(cw20::Cw20ReceiveMsg {
    //         msg: Some(
    //             to_binary(&Cw20HookMsg::Burn {
    //                 asset_weights: None,
    //                 redeem_mins: Some(new_assets),
    //             })
    //                 .unwrap(),
    //         ),
    //         sender: h("addr0000"),
    //         amount: Uint128(20_000_000),
    //     });
    //
    //     let env = mock_env(consts::cluster_token(), &[]);
    //     let res = handle(&mut deps, env, msg).unwrap();
    //     for log in res.log.iter() {
    //         println!("{}: {}", log.key, log.value);
    //     }
    //     assert_eq!(5, res.messages.len());
    // }

    #[test]
    fn reset_target() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        deps.querier
            .set_token_supply(consts::cluster_token(), 100_000_000)
            .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

        let new_assets = vec![
            AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            AssetInfo::Token {
                contract_addr: h("mGME"),
            },
        ];
        let new_targets: Vec<u32> = vec![10, 5, 35, 50];

        let msg = HandleMsg::ResetTarget {
            assets: new_assets.clone(),
            target: new_targets.clone(),
        };

        let env = mock_env(consts::owner(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();

        // mNFLX should still be in logs with target 0
        for log in res.log.iter() {
            println!("{}: {}", log.key, log.value);
        }
        assert_eq!(0, res.messages.len());
    }
}
