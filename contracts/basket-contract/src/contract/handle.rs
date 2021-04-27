use cosmwasm_std::{
    from_binary, log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, LogAttribute, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use error::bad_weight_values;

use crate::error;
use crate::ext_query::{query_cw20_balance_minus_staged, query_cw20_token_supply, query_price, query_mint_amount};
use crate::state::{
    read_config, save_config, stage_asset, unstage_asset, TargetAssetData,
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
use std::cmp::min;

/// Convenience function for creating inline HumanAddr
pub fn h(s: &str) -> HumanAddr {
    HumanAddr(s.to_string())
}

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

    // Using HumanAddr instead of AssetInfo for cw20
    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::Burn { asset_weights, redeem_mins } => {
                try_receive_burn(deps, env, &sender, &sent_asset, sent_amount, asset_weights, redeem_mins)
            }
            Cw20HookMsg::StageAsset {} => {
                try_receive_stage_asset(deps, env, &sender, &sent_asset, sent_amount)
            }
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
    sent_asset: &HumanAddr,
    sent_amount: Uint128,
    asset_weights: Option<Vec<Asset>>,
    redeem_mins:Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse::default())
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
    sent_asset: &HumanAddr,
    sent_amount: Uint128,
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
    if !assets.iter().any(|asset| match asset {
        AssetInfo::Token { contract_addr } => contract_addr == sent_asset,
        AssetInfo::NativeToken { denom } => &h(denom) == sent_asset,
    }) {
        return Err(error::not_component_cw20(sent_asset));
    }

    let sent_asset_info = AssetInfo::Token {
        contract_addr: sent_asset.clone(),
    };
    stage_asset(&mut deps.storage, sender, &sent_asset_info, sent_amount)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "receive:stage_asset"),
            log("sender", sender),
            log("asset", sent_asset),
            log("staged_amount", sent_amount),
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

    let target: Vec<Uint128> = target_asset_data
        .iter()
        .map(|x| Uint128(x.target as u128))
        .collect::<Vec<Uint128>>();

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
    let inv: Vec<Uint128> =
        asset_infos
            .iter()
            .map(|asset| match asset {
                AssetInfo::Token { contract_addr } =>
                    query_cw20_balance_minus_staged(&deps, &contract_addr, &env.contract.address),
                AssetInfo::NativeToken { denom } => query_cw20_balance_minus_staged(
                    &deps,
                    &h(denom),
                    &env.contract.address,
                ),
            })
            .collect::<StdResult<Vec<Uint128>>>()?;

    let c = asset_weights;


    // get current prices of each token via oracle
    let prices: Vec<String> = asset_infos
        .iter()
        .map(|asset| match asset {
            AssetInfo::Token { contract_addr } => query_price(&deps, &cfg.oracle, &contract_addr),
            AssetInfo::NativeToken { denom } => query_price(&deps, &cfg.oracle, &h(denom)),
        })
        .collect::<StdResult<Vec<String>>>()?;

    let basket_token_supply = query_cw20_token_supply(&deps, &basket_token)?;

    let mint_response = query_mint_amount(
        &deps,
        &cfg.penalty,
        basket_token_supply,
        inv,
        c,
        prices,
        target
    )?;

    let mint_total = mint_response.mint_tokens;

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

    let mut logs = vec![
        log("action", "mint"),
        log("sender", &env.message.sender),
        log("mint_total", mint_total),
    ];
    logs.extend(mint_response.log);

    // mint and send number of tokens to user
    Ok(HandleResponse {
        messages: vec![mint_msg],
        log: logs,
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
    asset: &AssetInfo,
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

    if !assets.iter().any(|x| x == asset) {
        return Err(error::not_component_asset(asset));
    }

    let curr_staged = read_staged_asset(&deps.storage, &env.message.sender, asset)?;
    let to_unstage = match amount {
        Some(amt) => {
            if *amt > curr_staged {
                return Err(error::insufficient_staged(
                    &env.message.sender,
                    asset,
                    *amt,
                    curr_staged,
                ));
            }
            *amt
        }
        None => curr_staged,
    };

    unstage_asset(&mut deps.storage, &env.message.sender, asset, to_unstage)?;

    // return asset
    let messages = if !to_unstage.is_zero() {
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: match asset {
                AssetInfo::Token { contract_addr } => contract_addr.clone(),
                AssetInfo::NativeToken { denom } => h(denom),
            },
            msg: to_binary(&Cw20HandleMsg::Transfer {
                amount: to_unstage.clone(),
                recipient: env.message.sender.clone(),
            })?,
            send: vec![],
        })]
    } else {
        vec![]
    };

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "unstage_asset"),
            log("asset", asset),
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
                amount: Uint128(10)
            },
            Asset {
                info: AssetInfo::Token {
                        contract_addr: h("mGOOG"),
                    },
                amount: Uint128(10)
            },
            Asset {
                info: AssetInfo::Token {
                        contract_addr: h("mMSFT"),
                    },
                amount: Uint128(10)
            },
            Asset {
                info: AssetInfo::Token {
                        contract_addr: h("mNFLX"),
                    },
                amount: Uint128(10)
            }

        ];

        let msg = HandleMsg::Receive(cw20::Cw20ReceiveMsg {
            msg: Some(
                to_binary(&Cw20HookMsg::Burn {
                    asset_weights: None,
                    redeem_mins:Some(new_assets),
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
