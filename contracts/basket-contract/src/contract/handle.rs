use cosmwasm_std::{
    from_binary, log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, LogAttribute, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};

use crate::penalty::{compute_penalty, compute_score};
use crate::state::{read_config, read_target, stage_asset, unstage_asset, PenaltyParams};
use crate::util::{fpdec_to_int, int_to_fpdec, vec_to_string};
use crate::{
    ext_query::{query_cw20_balance, query_price},
    test_helper::query_cw20_token_supply,
};
use crate::{
    msg::{Cw20HookMsg, HandleMsg},
    state::read_staged_asset,
};
use basket_math::{dot, sum, FPDecimal};

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
        HandleMsg::UnstageAsset { amount, asset } => try_unstage_asset(deps, env, &asset, amount),
        HandleMsg::ResetTarget { target } => try_reset_target(deps, env, &target),
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    let sender = cw20_msg.sender;
    let sent_asset = env.message.sender.clone();
    let sent_amount = cw20_msg.amount;

    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::Burn {
                num_tokens,
                asset_weights,
            } => try_receive_burn(
                deps,
                env,
                &sender,
                &sent_asset,
                sent_amount,
                num_tokens,
                asset_weights,
            ),
            Cw20HookMsg::StageAsset {} => {
                try_receive_stage_asset(deps, env, &sender, &sent_asset, sent_amount)
            }
        }
    } else {
        Err(StdError::generic_err(
            "Receive Hook - missing expected .msg in body",
        ))
    }
}

pub fn try_receive_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: &HumanAddr,
    sent_asset: &HumanAddr,
    sent_amount: Uint128,
    num_tokens: Uint128,
    asset_weights: Option<Vec<u32>>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // require that origin contract from Receive Hook is the associated Basket Token
    if *sent_asset != cfg.basket_token {
        return Err(StdError::unauthorized());
    }

    // check if (num_tokens) to be burnt <= sent_amount
    if num_tokens > sent_amount {
        return Err(StdError::generic_err(format!(
            "num of tokens to burn ({}) exceeds amount sent ({})",
            num_tokens, sent_amount
        )));
    }

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.basket_token.clone(),
        msg: to_binary(&Cw20HandleMsg::Burn { amount: num_tokens }).unwrap(),
        send: vec![],
    });

    let inv: Vec<FPDecimal> = cfg
        .assets
        .iter()
        .map(|asset| {
            int_to_fpdec(query_cw20_balance(&deps, &asset, &env.contract.address).unwrap())
        })
        .collect();

    let basket_token_supply = query_cw20_token_supply(&deps, &cfg.basket_token)?;

    let m_div_n = int_to_fpdec(num_tokens) / int_to_fpdec(basket_token_supply);

    let mut logs: Vec<LogAttribute> = Vec::new();
    let redeem_subtotals: Vec<FPDecimal> = match &asset_weights {
        Some(weights) => {
            // ensure the provided weights has the same dimension as our inventory
            if weights.len() != inv.len() {
                return Err(StdError::generic_err(format!(
                    "# assets in asset_weights ({}) does not match basket inventory ({})",
                    weights.len(),
                    inv.len()
                )));
            }
            let weights_sum = weights
                .iter()
                .fold(FPDecimal::zero(), |acc, &el| acc + FPDecimal::from(el));
            let r = weights // normalize weights vector
                .iter()
                .map(|&x| FPDecimal::from(x) / weights_sum)
                .collect();
            let prices: Vec<FPDecimal> = cfg
                .assets
                .iter()
                .map(|asset| query_price(&deps, &cfg.oracle, &asset).unwrap())
                .collect();
            let prod = dot(&inv, &prices) / dot(&r, &prices);
            let b: Vec<FPDecimal> = r
                .iter()
                .map(|&x| FPDecimal::one().mul(-1) * m_div_n * prod * x)
                .collect();

            // compute penalty
            let score = compute_score(&inv, &b, &prices, &read_target(&deps.storage)?);
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
    let (redeem_totals, redeem_roundoffs): (Vec<Uint128>, Vec<FPDecimal>) =
        redeem_subtotals.iter().map(|&x| fpdec_to_int(x)).unzip();

    let transfer_msgs: Vec<CosmosMsg> = redeem_totals
        .iter()
        .zip(cfg.assets.iter())
        .map(|(amt, asset)| {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: asset.clone(),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    amount: amt.clone(),
                    recipient: sender.clone(),
                })
                .unwrap(),
                send: vec![],
            })
        })
        .collect();

    Ok(HandleResponse {
        messages: vec![vec![burn_msg], transfer_msgs].concat(),
        log: vec![
            vec![
                log("action", "receive:burn"),
                log("sender", sender),
                log("sent_asset", sent_asset),
                log("sent_tokens", sent_amount),
                log("tokens_burned", num_tokens),
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

pub fn try_receive_stage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: &HumanAddr,
    sent_asset: &HumanAddr,
    sent_amount: Uint128,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // if sent asset is not a component asset of basket, reject
    if !cfg.assets.iter().any(|asset| asset == sent_asset) {
        return Err(StdError::generic_err(format!(
            "asset ({}) is not a component asset of basket",
            sent_asset
        )));
    }

    stage_asset(&mut deps.storage, sender, sent_asset, sent_amount)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "receive:stage_asset"),
            log("sender", sender),
            log("asset", sent_asset),
            log("amount", sent_amount),
        ],
        data: None,
    })
}

/// May be called by the Basket contract owner to reset the target
pub fn try_reset_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    target: &Vec<u32>,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_amounts: &Vec<Uint128>,
    min_tokens: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;
    let target = read_target(&deps.storage)?;

    // ensure that all tokens in asset_amounts have been staged beforehand
    for (asset, amount) in cfg.assets.iter().zip(asset_amounts) {
        let staged = read_staged_asset(&deps.storage, &env.message.sender, asset).unwrap();
        if *amount < staged {
            return Err(StdError::generic_err(format!(
                "insufficient asset {} staged, {} < {} (requested)",
                asset, amount, staged
            )));
        }
        unstage_asset(&mut deps.storage, &env.message.sender, &asset, *amount)?;
    }
    let c = asset_amounts.iter().map(|&x| int_to_fpdec(x)).collect();

    // get current balances of each token (inventory)
    let inv: Vec<FPDecimal> = cfg
        .assets
        .iter()
        .map(|asset| {
            int_to_fpdec(query_cw20_balance(&deps, &asset, &env.contract.address).unwrap())
        })
        .collect();

    // get current prices of each token via oracle
    let prices: Vec<FPDecimal> = cfg
        .assets
        .iter()
        .map(|asset| query_price(&deps, &cfg.oracle, &asset).unwrap())
        .collect();

    // compute penalty
    let score = compute_score(&inv, &c, &prices, &target);
    let PenaltyParams {
        a_pos,
        s_pos,
        a_neg,
        s_neg,
    } = cfg.penalty_params;
    let penalty = compute_penalty(score, a_pos, s_pos, a_neg, s_neg);
    let basket_token_supply = query_cw20_token_supply(&deps, &cfg.basket_token)?;

    // compute number of new tokens
    let mint_subtotal =
        penalty * dot(&c, &prices) / dot(&inv, &prices) * int_to_fpdec(basket_token_supply);

    let (mint_total, mint_roundoff) = fpdec_to_int(mint_subtotal); // the fraction part is kept inside basket

    if let Some(m) = min_tokens {
        if mint_total < *m {
            return Err(StdError::generic_err(format!(
                "transaction aborted: transaction would mint {}, which is less than min_tokens specified: {}",
                mint_total.0, m
            )));
        }
    }

    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.basket_token.clone(),
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

pub fn try_unstage_asset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset: &HumanAddr,
    amount: &Option<Uint128>,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    // if sent asset is not a component asset of basket, reject
    if !cfg.assets.iter().any(|x| asset == x) {
        return Err(StdError::generic_err(format!(
            "asset ({}) is not a component asset of basket",
            asset
        )));
    }

    let curr_staged = read_staged_asset(&deps.storage, &env.message.sender, asset)?;
    let to_unstage = match amount {
        Some(amt) => {
            if *amt > curr_staged {
                return Err(StdError::generic_err(format!(
                    "can't unstage more than available: {} (requested) > {} (staged)",
                    amt, curr_staged
                )));
            }
            *amt
        }
        None => curr_staged,
    };

    unstage_asset(&mut deps.storage, &env.message.sender, asset, to_unstage)?;

    // return asset
    let messages = if !to_unstage.is_zero() {
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset.clone(),
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
        let msg = HandleMsg::Mint {
            asset_amounts: vec![
                Uint128(125_000_000), // mAAPL
                Uint128::zero(),      // mGOOG
                Uint128(149_000_000), // mMSFT
                Uint128(50_090_272),  // mNFLX
            ],
            min_tokens: None,
        };

        let env = mock_env(consts::owner(), &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        for log in res.log.iter() {
            println!("{}: {}", log.key, log.value);
        }
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn burn() {
        let (mut deps, _init_res) = mock_init();
        mock_querier_setup(&mut deps);

        deps.querier
            .set_token_supply(consts::basket_token(), 100_000_000)
            .set_token_balance(consts::basket_token(), "addr0000", 20_000_000);

        let msg = HandleMsg::Receive(cw20::Cw20ReceiveMsg {
            msg: Some(
                to_binary(&Cw20HookMsg::Burn {
                    asset_weights: None,
                    num_tokens: Uint128(20_000_000),
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
        assert_eq!(1, res.messages.len());
    }
}
