use cosmwasm_std::{
    from_binary, log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use error::bad_weight_values;

use crate::contract::query_basket_state;
use crate::error;
use crate::ext_query::{query_collector_contract_address, query_mint_amount, query_redeem_amount, ExtQueryMsg};
use crate::state::{read_config, save_config, stage_asset, unstage_asset, TargetAssetData};
use crate::state::{read_target_asset_data, save_target_asset_data};
use crate::util::vec_to_string;
use nebula_protocol::cluster::{Cw20HookMsg, HandleMsg};
use crate::state::read_staged_asset;
use terraswap::asset::{Asset, AssetInfo};
use basket_math::FPDecimal;
use std::str::FromStr;
use nebula_protocol::collector::HandleMsg as CollectorHandleMsg;

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
        },
        HandleMsg::ResetCompositionOracle { composition_oracle } => try_reset_composition_oracle(deps, env, &composition_oracle),
        HandleMsg::ResetPenalty { penalty } => try_reset_penalty(deps, env, &penalty),
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, env, &owner),
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
            Cw20HookMsg::Burn { asset_amounts } => {
                try_receive_burn(deps, env, &sender, &asset, asset_amounts)
            }
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
    asset_amounts: Option<Vec<Asset>>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let basket_token = cfg
        .basket_token
        .clone()
        .ok_or_else(|| error::basket_token_not_set())?;

    let basket_state = query_basket_state(&deps, &env.contract.address)?;

    let prices = basket_state.prices;
    let basket_token_supply = basket_state.outstanding_balance_tokens;
    let inv = basket_state.inv;
    let target = basket_state.target;

    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let asset_infos = target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let sent_asset = match &asset.info {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom: _ } => {
            return Err(StdError::unauthorized());
        }
    };
    let sent_amount = asset.amount;

    let asset_amounts: Vec<Uint128> = match &asset_amounts {
        Some(weights) => {
            let mut vec: Vec<Uint128> = Vec::new();
            for i in 0..asset_infos.len() {
                for j in 0..weights.len() {
                    if weights[j].info.clone() == asset_infos[i].clone() {
                        vec.push(weights[j].amount);
                        break;
                    }
                }
            }
            vec
        }
        None => vec![],
    };

    // require that origin contract from Receive Hook is the associated Basket Token
    if *sent_asset != basket_token {
        return Err(StdError::unauthorized());
    }

    let max_tokens = sent_amount.clone();


    let (collector_address, fee_rate) =
        query_collector_contract_address(&deps, &cfg.factory)?;
    let fee_rate: FPDecimal = FPDecimal::from_str(&*fee_rate)?;
    let keep_rate: FPDecimal = FPDecimal::one() - fee_rate;

    let token_cap = Uint128((FPDecimal::from(max_tokens.u128()) * keep_rate).into());

    let redeem_response = query_redeem_amount(
        &deps,
        &cfg.penalty,
        env.block.height,
        basket_token_supply,
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

    let mint_to_sender = (max_tokens - token_cost)?;

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

    messages.push(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: basket_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Burn {
                amount: max_tokens.clone(),
            })?,
            send: vec![],
        })
    );

    // afterwards, notify the penalty contract that this update happened so
    // it can make stateful updates...
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.penalty.clone(),
        msg: to_binary(&ExtQueryMsg::Redeem {
            block_height: env.block.height,
            basket_token_supply,
            inventory: inv,
            max_tokens,
            redeem_asset_amounts: asset_amounts.clone(),
            asset_prices: prices,
            target_weights: target
        })?,
        send: vec![],
    }));

    // send remaining basket tokens back to the sender
    if mint_to_sender > Uint128::zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: basket_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Mint {
                amount: mint_to_sender,
                recipient: sender.clone(),
            })?,
            send: vec![],
        }));
    }

    // send fees to collector contract
    if fee_amt > Uint128::zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: basket_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Mint {
                amount: fee_amt,
                recipient: collector_address.clone(),
            })?,
            send: vec![],
        }));
    }

    if redeem_response.penalty > Uint128::zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collector_address.clone(),
            msg: to_binary(&CollectorHandleMsg::RecordPenalty {
                asset_address: basket_token.clone(),
                reward_owner: env.message.sender.clone(),
                penalty_amount: redeem_response.penalty,
            })?,
            send: vec![],
        }));
    }



    Ok(HandleResponse {
        messages,
        log: vec![
            vec![
                log("action", "receive:burn"),
                log("sender", sender),
                log("sent_asset", sent_asset),
                log("sent_tokens", sent_amount),
                log("burn_amount", max_tokens),
                log("token_cost", token_cost),
                log("returned_to_sender", mint_to_sender),
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
    // allow removal / adding

    let cfg = read_config(&deps.storage)?;
    if let None = cfg.basket_token {
        return Err(error::basket_token_not_set());
    }
    // check permission
    if (env.message.sender != cfg.owner) && (env.message.sender != cfg.composition_oracle) {
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

// Changes the composotion oracle contract for this basket
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
        log: vec![log("action", "reset_composition_oracle"), log("composition_oracle", &composition_oracle)],
        data: None,
    })
}

// Changes the penalty contract for this basket
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
     May be called by the Basket contract owner to reset the owner
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
    // if let Some(token) = cfg.basket_token {
    //     return Err(error::basket_token_already_set(&token));
    // }

    let mut new_cfg = cfg.clone();
    new_cfg.owner = owner.clone();
    save_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_try_reset_owner"),
            log("basket_token", &owner),
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
    let basket_state = query_basket_state(&deps, &env.contract.address)?;

    let prices = basket_state.prices;
    let basket_token_supply = basket_state.outstanding_balance_tokens;
    let inv = basket_state.inv;
    let target = basket_state.target;

    let cfg = read_config(&deps.storage)?;
    let target_asset_data = read_target_asset_data(&deps.storage)?;
    let asset_infos = &target_asset_data
        .iter()
        .map(|x| x.asset.clone())
        .collect::<Vec<_>>();

    let basket_token = cfg
        .basket_token
        .clone()
        .ok_or_else(|| error::basket_token_not_set())?;

    // accommmodate inputs: subsets of target assets vector

    let mut new_asset_weights = vec![Uint128(0); asset_infos.len()];

    // list of 0's for vector (size of asset_infos) -> replace in the for loop
    let asset_weights: Vec<Uint128> = {
        for i in 0..asset_infos.len() {
            for weight in asset_amounts {
                if weight.info.clone() == asset_infos[i].clone() {
                    new_asset_weights[i] = weight.amount;
                    break;
                }
            }
            // 0 if else
        }
        new_asset_weights
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

    let c = asset_weights;

    let mint_to_sender;

    let mut extra_logs = vec![];
    let mut messages = vec![];
    // do a regular mint
    if basket_token_supply != Uint128::zero() {
        let mint_response = query_mint_amount(
            &deps,
            &cfg.penalty.clone(),
            env.block.height,
            basket_token_supply,
            inv.clone(),
            c.clone(),
            prices.clone(),
            target.clone(),
        )?;
        let mint_total = mint_response.mint_tokens;

        let (collector_address, fee_rate) =
            query_collector_contract_address(&deps, &cfg.factory)?;
        let fee_rate = FPDecimal::from_str(&*fee_rate)?;

        // Decimal doesn't give the ability to subtract...
        mint_to_sender = Uint128((FPDecimal::from(mint_total.u128()) * (FPDecimal::one() - fee_rate)).into());
        let protocol_fee = (mint_total - mint_to_sender)?;

        // afterwards, notify the penalty contract that this update happened so
        // it can make stateful updates...
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.penalty.clone(),
            msg: to_binary(&ExtQueryMsg::Mint {
                block_height: env.block.height,
                basket_token_supply,
                inventory: inv,
                mint_asset_amounts: c,
                asset_prices: prices,
                target_weights: target
            })?,
            send: vec![],
        }));

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: basket_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Mint {
                amount: protocol_fee,
                recipient: collector_address.clone(),
            })?,
            send: vec![],
        }));

        if mint_response.penalty > Uint128::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: collector_address.clone(),
                msg: to_binary(&CollectorHandleMsg::RecordPenalty {
                    asset_address: basket_token.clone(),
                    reward_owner: env.message.sender.clone(),
                    penalty_amount: mint_response.penalty,
                })?,
                send: vec![],
            }));
        }


        extra_logs = mint_response.log;
        extra_logs.push(log("fee_amt", protocol_fee))
    } else {
        // basket has no basket tokens -- basket is empty and needs to be initialized
        // attempt to initialize it with min_tokens as the number of basket tokens
        // and the mint basket c as the initial assets
        // c is required to be in ratio with the target weights
        if let Some(proposed_mint_total) = min_tokens {
            let mut val = 0;
            for i in 0..c.len() {
                if inv[i].u128() % c[i].u128() != 0u128 {
                    return Err(StdError::generic_err(
                        "Initial basket assets must be in target weights",
                    ));
                }
                let div = inv[i].u128() / c[i].u128();
                if val == 0 {
                    val = div;
                }
                if div != val {
                    return Err(StdError::generic_err(
                        "Initial basket assets must be in target weights",
                    ));
                }
            }

            mint_to_sender = *proposed_mint_total;
        } else {
            return Err(StdError::generic_err(
                "Basket is uninitialized. \
            To initialize it with your mint basket, \
            provide min_tokens as the amount of basket tokens you want to start with.",
            ));
        }
    }

    if let Some(min) = min_tokens {
        if mint_to_sender < *min {
            return Err(error::below_min_tokens(mint_to_sender, *min));
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

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: basket_token.clone(),
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
    use nebula_protocol::cluster::{HandleMsg};

    // #[test]
    // fn mint() {
    //     let (mut deps, _init_res) = mock_init();
    //     mock_querier_setup(&mut deps);
    //
    //     // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
    //     // ---
    //     // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
    //     // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
    //     // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
    //     // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389
    //
    //     // The set token balance should include the amount we would also like to stage
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
    //                 contract_addr: h("mAAPL"),
    //             },
    //             amount: Uint128(125_000_000),
    //         },
    //         Asset {
    //             info: AssetInfo::Token {
    //                 contract_addr: h("mGOOG"),
    //             },
    //             amount: Uint128::zero(),
    //         },
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
    //                 AssetInfo::NativeToken { .. } => return,
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
    //         .set_token_balance("wBTC", consts::basket_token(), 1_000_000)
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
    //         .set_token_supply(consts::basket_token(), 100_000_000)
    //         .set_token_balance(consts::basket_token(), "addr0000", 20_000_000);
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
    //     let env = mock_env(consts::basket_token(), &[]);
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
