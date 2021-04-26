use cosmwasm_std::{
    from_binary, log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, LogAttribute, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use error::bad_weight_values;

use crate::error;
use crate::ext_query::{
    query_cw20_balance_minus_staged, query_cw20_token_supply, query_native_balance_minus_staged,
    query_price,
};
use crate::state::{
    read_config, save_config, stage_asset, unstage_asset, PenaltyParams, TargetAssetData,
};
use crate::util::{fpdec_to_int, int_to_fpdec, vec_to_string};
use crate::{
    msg::{Cw20HookMsg, HandleMsg},
    state::read_staged_asset,
};
use crate::{
    penalty::{compute_diff, compute_penalty, compute_score},
    state::{read_target_asset_data, save_target_asset_data},
};
use basket_math::{dot, sum, FPDecimal};
use terraswap::asset::{Asset, AssetInfo};

/*
    Match the incoming message to the right category: receive, mint,
    unstage, reset_target, or  set basket token
*/
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::Mint {
            asset_amounts,
            min_tokens,
        } => try_mint(deps, env, &asset_amounts, &min_tokens),
        HandleMsg::UnstageAsset { amount, asset } => try_unstage_asset(deps, env, &asset, &amount),
        HandleMsg::ResetTarget { assets, target } => try_reset_target(deps, env, &assets, &target),
        HandleMsg::_SetBasketToken { basket_token } => {
            try_set_basket_token(deps, env, &basket_token)
        } // HandleMsg::AddAssetType {asset} => try_add_asset_type(deps, env, asset),
        HandleMsg::StageNativeAsset { asset } => {
            // only native token can be deposited directly
            if !asset.is_native_token() {
                return Err(StdError::unauthorized());
            }

            // Check the actual deposit happens
            asset.assert_sent_native_token_balance(&env)?;

            let sender = &env.message.sender.clone();

            try_receive_stage_asset(deps, env, sender, &asset)
        }
    }
}

/*
    Receives CW20 tokens which can either be cluster tokens that are burned
    to receive assets, or assets that can be staged for the process of minting
    cluster tokens.
*/
pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    let sender = cw20_msg.sender;
    let sent_asset = env.message.sender.clone();
    let sent_amount = cw20_msg.amount;
    let asset = Asset {
        info: AssetInfo::Token {
            contract_addr: sent_asset,
        },
        amount: sent_amount,
    };

    // Using HumanAddr instead of AssetInfo for cw20
    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::Burn {
                asset_weights,
                redeem_mins,
            } => try_receive_burn(deps, env, &sender, &asset, asset_weights, redeem_mins),
            Cw20HookMsg::StageAsset {} => try_receive_stage_asset(deps, env, &sender, &asset),
        }
    } else {
        Err(error::missing_cw20_msg())
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
    sender: &HumanAddr,
    asset: &Asset,
    asset_weights: Option<Vec<Asset>>,
    redeem_mins: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let basket_token = cfg
        .basket_token
        .clone()
        .ok_or_else(|| error::basket_token_not_set())?;

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let target = target_asset_data
        .iter()
        .map(|x| x.target)
        .collect::<Vec<_>>();
    let sent_asset = match &asset.info {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom: _ } => {
            return Err(StdError::unauthorized());
        }
    };
    let sent_amount = asset.amount;

    // Reorder asset_weights according to ordering of target assets
    let ordered_min_redeem: Option<Vec<Uint128>> = match &redeem_mins {
        Some(mins) => {
            let mut vec: Vec<Uint128> = Vec::new();
            for i in 0..assets.len() {
                for j in 0..mins.len() {
                    if mins[j].info.clone() == assets[i].clone() {
                        vec.push(mins[j].amount);
                        break;
                    }
                }
            }
            Some(vec)
        }
        None => None,
    };

    let asset_weights: Option<Vec<Uint128>> = match &asset_weights {
        Some(weights) => {
            let mut vec: Vec<Uint128> = Vec::new();
            for i in 0..assets.len() {
                for j in 0..weights.len() {
                    if weights[j].info.clone() == assets[i].clone() {
                        vec.push(weights[j].amount);
                        break;
                    }
                }
            }
            Some(vec)
        }
        None => None,
    };

    // require that origin contract from Receive Hook is the associated Basket Token
    if *sent_asset != basket_token {
        return Err(StdError::unauthorized());
    }

    let burn_amount = sent_amount.clone();

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: basket_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Burn {
            amount: burn_amount.clone(),
        })?,
        send: vec![],
    });

    let inv: Vec<FPDecimal> =
        assets
            .iter()
            .map(|asset| match asset {
                AssetInfo::Token { contract_addr } => int_to_fpdec(
                    query_cw20_balance_minus_staged(&deps, &contract_addr, &env.contract.address)?,
                ),
                AssetInfo::NativeToken { denom } => int_to_fpdec(
                    query_native_balance_minus_staged(&deps, denom, &env.contract.address)?,
                ),
            })
            .collect::<StdResult<Vec<FPDecimal>>>()?;

    let basket_token_supply = query_cw20_token_supply(&deps, &basket_token)?;

    let m_div_n = int_to_fpdec(burn_amount)? / int_to_fpdec(basket_token_supply)?;

    let mut logs: Vec<LogAttribute> = Vec::new();
    let redeem_subtotals: Vec<FPDecimal> = match &asset_weights {
        Some(weights) => {
            // ensure the provided weights has the same dimension as our inventory
            if weights.len() != inv.len() {
                return Err(error::bad_weight_dimensions(weights.len(), inv.len()));
            }
            let weights_sum = weights.iter().fold(FPDecimal::zero(), |acc, &el| {
                acc + FPDecimal::from(el.u128())
            });

            let r = weights // normalize weights vector
                .iter()
                .map(|&x| FPDecimal::from(x.u128()) / weights_sum)
                .collect();

            let prices: Vec<FPDecimal> = assets
                .iter()
                .map(|asset_info| query_price(&deps, &cfg.oracle, asset_info))
                .collect::<StdResult<Vec<FPDecimal>>>()?;

            let prod = dot(&inv, &prices) / dot(&r, &prices);
            let b: Vec<FPDecimal> = r.iter().map(|&x| m_div_n * prod * x).collect();
            let neg_b: Vec<FPDecimal> = b.iter().map(|&x| FPDecimal::from(-1i128) * x).collect();

            // compute score
            let diff = compute_diff(&inv, &neg_b, &prices, &target);
            let score = (sum(&diff) / dot(&b, &prices)).div(2i128);

            let PenaltyParams {
                a_pos,
                s_pos,
                a_neg,
                s_neg,
            } = cfg.penalty_params;
            let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);
            logs.push(log("score", score));
            logs.push(log("penalty", penalty));
            b.iter().map(|&x| penalty * x).collect()
        }
        None => inv.iter().map(|&x| m_div_n * x).collect(),
    };

    // convert reward into Uint128 -- truncate decimal as roundoff
    let redeem: Vec<(Uint128, FPDecimal)> = redeem_subtotals
        .iter()
        .map(|&x| fpdec_to_int(x))
        .collect::<StdResult<Vec<(Uint128, FPDecimal)>>>()?;

    let (redeem_totals, redeem_roundoffs): (Vec<Uint128>, Vec<FPDecimal>) =
        redeem.iter().cloned().unzip();

    if let Some(order_mins) = ordered_min_redeem {
        for i in 0..redeem_totals.len() {
            if redeem_totals[i] < order_mins[i] {
                return Err(error::below_min_tokens(redeem_totals[i], order_mins[i]));
            }
        }
    }

    let transfer_msgs: Vec<CosmosMsg> = redeem_totals
        .iter()
        .zip(assets.iter())
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

    Ok(HandleResponse {
        messages: vec![vec![burn_msg], transfer_msgs].concat(),
        log: vec![
            vec![
                log("action", "receive:burn"),
                log("sender", sender),
                log("sent_asset", sent_asset),
                log("sent_tokens", sent_amount),
                log("burn_amount", burn_amount),
                log(
                    "asset_weights",
                    match &asset_weights {
                        Some(v) => vec_to_string(v),
                        None => "".to_string(),
                    },
                ),
                log("redeem_totals", vec_to_string(&redeem_totals)),
                log("redeem_roundoffs", vec_to_string(&redeem_roundoffs)),
            ],
            logs,
        ]
        .concat(),
        data: None,
    })
}

/*
    Receives an asset which is part of the cluster and stages it such that
    the contract records the sender of the asset and the balance sent. This
    balance is later looked up when this sender wants to mint cluster tokens from
    the a number of staged assets.
*/
pub fn try_receive_stage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    sender: &HumanAddr,
    staged_asset: &Asset,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    if let None = cfg.basket_token {
        return Err(error::basket_token_not_set());
    }

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    // if sent asset is not a component asset of basket, reject
    if !assets.iter().any(|asset| *asset == staged_asset.info) {
        return Err(error::not_component_cw20(&staged_asset.info));
    }

    stage_asset(
        &mut deps.storage,
        sender,
        &staged_asset.info,
        staged_asset.amount,
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "receive:stage_asset"),
            log("sender", sender),
            log("asset", staged_asset.info.clone()),
            log("staged_amount", staged_asset.amount),
        ],
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
    target: &Vec<u32>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    if let None = cfg.basket_token {
        return Err(error::basket_token_not_set());
    }
    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    //TODO: Make sure all assets in new asset vector actually exist
    let provided: u32 = target.clone().iter().sum();
    if provided != 100 {
        return Err(bad_weight_values(provided));
    }

    if target.len() != assets.len() {
        return Err(error::bad_weight_dimensions(target.len(), assets.len()));
    }
    let mut asset_data: Vec<TargetAssetData> = Vec::new();
    for i in 0..target.len() {
        let asset_elem = TargetAssetData {
            asset: assets[i].clone(),
            // asset: AssetInfo::Token {
            //     contract_addr: assets[i].clone(),
            // },
            target: target[i].clone(),
        };
        asset_data.push(asset_elem);
    }

    let updated_assets = asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let updated_target = asset_data.iter().map(|x| x.target).collect::<Vec<_>>();

    let prev_asset_data = read_target_asset_data(&deps.storage)?;
    let prev_assets = prev_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let prev_target = prev_asset_data.iter().map(|x| x.target).collect::<Vec<_>>();

    save_target_asset_data(&mut deps.storage, &asset_data)?;

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

/*
     May be called by the Basket contract owner to set the basket token for first time
*/
pub fn try_set_basket_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    basket_token: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // check permission
    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    // check if already set
    if let Some(token) = cfg.basket_token {
        return Err(error::basket_token_already_set(&token));
    }

    let mut new_cfg = cfg.clone();
    new_cfg.basket_token = Some(basket_token.clone());
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_set_basket_token"),
            log("basket_token", &basket_token),
        ],
        data: None,
    })
}

/*
    Tries to mint cluster tokens from the asset amounts given.
    Throws error if there can only be less than 'min_tokens' minted from the assets.
    Note that the corresponding asset amounts need to be staged before in order to
    successfully mint tokens.
*/
pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_amounts: &Vec<Asset>,
    min_tokens: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let asset_infos = &target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    let target = target_asset_data
        .iter()
        .map(|x| x.target)
        .collect::<Vec<_>>();
    let basket_token = cfg
        .basket_token
        .clone()
        .ok_or_else(|| error::basket_token_not_set())?;

    let asset_weights: Vec<Uint128> = {
        let mut vec: Vec<Uint128> = Vec::new();
        for i in 0..asset_infos.len() {
            for weight in asset_amounts {
                if weight.info.clone() == asset_infos[i].clone() {
                    vec.push(weight.amount);
                    break;
                }
            }
        }
        vec
    };

    // ensure that all tokens in asset_amounts have been staged beforehand
    for asset in asset_amounts {
        let staged = read_staged_asset(&deps.storage, &env.message.sender, &asset.info)?;
        //println!("asset {} amount {} staged {}", asset, amount, staged);
        if asset.amount > staged {
            return Err(error::insufficient_staged(
                &env.message.sender,
                &asset.info,
                asset.amount,
                staged,
            ));
        }
    }

    // get current balances of each token (inventory)
    let inv: Vec<FPDecimal> =
        asset_infos
            .iter()
            .map(|asset| match asset {
                AssetInfo::Token { contract_addr } => int_to_fpdec(
                    query_cw20_balance_minus_staged(&deps, &contract_addr, &env.contract.address)?,
                ),
                AssetInfo::NativeToken { denom } => int_to_fpdec(
                    query_native_balance_minus_staged(&deps, denom, &env.contract.address)?,
                ),
            })
            .collect::<StdResult<Vec<FPDecimal>>>()?;

    let c = asset_weights
        .iter()
        .map(|x| int_to_fpdec(x.clone()))
        .collect::<StdResult<Vec<FPDecimal>>>()?;

    // get current prices of each token via oracle
    let prices: Vec<FPDecimal> = asset_infos
        .iter()
        .map(|asset_info| query_price(&deps, &cfg.oracle, asset_info))
        .collect::<StdResult<Vec<FPDecimal>>>()?;

    // compute penalty
    let score = compute_score(&inv, &c, &prices, &target).div(2i128);

    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);

    let basket_token_supply = query_cw20_token_supply(&deps, &basket_token)?;

    // compute number of new tokens
    let mint_subtotal =
        penalty * dot(&c, &prices) / dot(&inv, &prices) * int_to_fpdec(basket_token_supply)?;

    let (mint_total, mint_roundoff) = fpdec_to_int(mint_subtotal)?; // the fraction part is kept inside basket

    if let Some(min) = min_tokens {
        if mint_total < *min {
            return Err(error::below_min_tokens(mint_total, *min));
        }
    }

    // Unstage after everything works
    for asset in asset_amounts {
        unstage_asset(
            &mut deps.storage,
            &env.message.sender,
            &asset.info,
            asset.amount,
        )?;
    }

    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: basket_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Mint {
            amount: mint_total,
            recipient: env.message.sender.clone(),
        })?,
        send: vec![],
    });

    // mint and send number of tokens to user
    Ok(HandleResponse {
        messages: vec![mint_msg],
        log: vec![
            log("action", "mint"),
            log("sender", &env.message.sender),
            log("score", score),
            log("penalty", penalty),
            log("mint_total", mint_total),
            log("mint_roundoff", mint_roundoff),
        ],
        data: None,
    })
}

/*
    Tries to unstage the given asset by the given amount by giving back
    the user the requested amount of asset only if the user has enough
    previously staged asset. Throws an error otherwise.
*/
pub fn try_unstage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_info: &AssetInfo,
    amount: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let assets = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();
    // if sent asset is not a component asset of basket, reject
    // if !assets.iter().any(|x| match x {
    //     AssetInfo::Token { contract_addr } => contract_addr == asset,
    //     AssetInfo::NativeToken { denom } => &h(denom) == asset,
    // })

    if !assets.iter().any(|x| x == asset_info) {
        return Err(error::not_component_asset(asset_info));
    }

    let curr_staged = read_staged_asset(&deps.storage, &env.message.sender, asset_info)?;
    let to_unstage = match amount {
        Some(amt) => {
            if *amt > curr_staged {
                return Err(error::insufficient_staged(
                    &env.message.sender,
                    asset_info,
                    *amt,
                    curr_staged,
                ));
            }
            *amt
        }
        None => curr_staged,
    };

    unstage_asset(
        &mut deps.storage,
        &env.message.sender,
        asset_info,
        to_unstage,
    )?;

    // return asset
    let messages = if !to_unstage.is_zero() {
        let asset = Asset {
            info: asset_info.clone(),
            amount: to_unstage.clone(),
        };

        // TODO: Check if sender field is correct here (recipient should be sender.clone())
        vec![asset.into_msg(
            &deps,
            env.contract.address.clone(),
            env.message.sender.clone(),
        )?]
    } else {
        vec![]
    };

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "unstage_asset"),
            log("asset", asset_info),
            log("amount", to_unstage),
        ],
        data: None,
    })
}

#[cfg(test)]
mod tests {

    use crate::test_helper::*;
    use cw20::Cw20ReceiveMsg;
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
        deps.querier
            .set_token_balance("mAAPL", consts::basket_token(), 7_290_053_159)
            .set_token_balance("mGOOG", consts::basket_token(), 319_710_128)
            .set_token_balance("mMSFT", consts::basket_token(), 14_219_281_228)
            .set_token_balance("mNFLX", consts::basket_token(), 224_212_221)
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
        let res = handle(&mut deps, env, mint_msg.clone());
        match res {
            Err(..) => (),
            _ => panic!("requires staging"),
        }

        for asset in asset_amounts {
            let env = mock_env(
                match asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { denom } => h(&denom),
                },
                &[],
            );
            let stage_asset_msg = HandleMsg::Receive(Cw20ReceiveMsg {
                sender: h("addr0000"),
                msg: Some(to_binary(&Cw20HookMsg::StageAsset {}).unwrap()),
                amount: asset.amount,
            });
            handle(&mut deps, env, stage_asset_msg).unwrap();
        }

        let env = mock_env(h("addr0000"), &[]);
        let res = handle(&mut deps, env, mint_msg).unwrap();

        // for _log in res.log.iter() {
        //     //println!("{}: {}", log.key, log.value);
        // }
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn mint_with_native_stage() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
        // ---
        // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
        // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
        // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
        // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389
        deps.querier
            .set_token_balance("wBTC", consts::basket_token(), 1_000_000)
            .set_token_balance("LUNA", consts::basket_token(), 2_000_000)
            .set_oracle_prices(vec![
                ("wBTC", Decimal::from_str("30000.00").unwrap()),
                ("LUNA", Decimal::from_str("15.00").unwrap()),
            ]);

        let asset_amounts = vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("wBTC"),
                },
                amount: Uint128(125_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::zero(),
            },
        ];
        let mint_msg = HandleMsg::Mint {
            asset_amounts: asset_amounts.clone(),
            min_tokens: None,
        };

        let env = mock_env(h("addr0000"), &[]);
        let res = handle(&mut deps, env, mint_msg.clone());
        match res {
            Err(..) => (),
            _ => panic!("requires staging"),
        }

        for asset in asset_amounts {
            let env = mock_env(
                match asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { denom } => h(&denom),
                },
                &[],
            );
            let stage_asset_msg = HandleMsg::Receive(Cw20ReceiveMsg {
                sender: h("addr0000"),
                msg: Some(to_binary(&Cw20HookMsg::StageAsset {}).unwrap()),
                amount: asset.amount,
            });
            handle(&mut deps, env, stage_asset_msg).unwrap();
        }

        let env = mock_env(h("addr0000"), &[]);
        let res = handle(&mut deps, env, mint_msg).unwrap();

        // for _log in res.log.iter() {
        //     //println!("{}: {}", log.key, log.value);
        // }
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn burn() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        deps.querier
            .set_token_supply(consts::basket_token(), 100_000_000)
            .set_token_balance(consts::basket_token(), "addr0000", 20_000_000);

        let new_assets = vec![
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
                amount: Uint128(10),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128(10),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128(10),
            },
        ];

        let msg = HandleMsg::Receive(cw20::Cw20ReceiveMsg {
            msg: Some(
                to_binary(&Cw20HookMsg::Burn {
                    asset_weights: None,
                    redeem_mins: Some(new_assets),
                })
                .unwrap(),
            ),
            sender: h("addr0000"),
            amount: Uint128(20_000_000),
        });

        let env = mock_env(consts::basket_token(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        for log in res.log.iter() {
            println!("{}: {}", log.key, log.value);
        }
        assert_eq!(5, res.messages.len());
    }

    #[test]
    fn reset_target() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        deps.querier
            .set_token_supply(consts::basket_token(), 100_000_000)
            .set_token_balance(consts::basket_token(), "addr0000", 20_000_000);

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
                contract_addr: h("mNFLX"),
            },
            AssetInfo::Token {
                contract_addr: h("GME"),
            },
        ];
        let new_targets: Vec<u32> = vec![10, 5, 30, 5, 50];

        let msg = HandleMsg::ResetTarget {
            assets: new_assets.clone(),
            target: new_targets.clone(),
        };

        let env = mock_env(consts::owner(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();

        for log in res.log.iter() {
            println!("{}: {}", log.key, log.value);
        }
        assert_eq!(0, res.messages.len());
    }
}
