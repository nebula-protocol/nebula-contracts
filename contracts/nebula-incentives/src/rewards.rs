use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, Order,
    Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::state::{
    contributions_read, contributions_to_pending_rewards, pool_info_read, pool_info_store,
    read_config, read_current_n, read_from_pool_bucket, read_pending_rewards, store_current_n,
    store_pending_rewards, Config, PoolInfo,
};

use nebula_protocol::incentives::{ExtHandleMsg, PoolType};

use cw20::Cw20HandleMsg;

// anybody can deposit rewards...
pub fn deposit_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    // pool_type, asset_address, amount
    rewards: Vec<(u16, HumanAddr, Uint128)>,
    rewards_amount: Uint128,
) -> HandleResult {
    let cfg = read_config(&deps.storage)?;
    let n = read_current_n(&deps.storage)?;

    for (pool_type, asset_token, amount) in rewards.iter() {
        if !PoolType::ALL_TYPES.contains(&pool_type) {
            return Err(StdError::generic_err("pool type not found"));
        }
        let mut pool_info: PoolInfo =
            read_from_pool_bucket(&pool_info_read(&deps.storage, *pool_type, n), &asset_token);
        pool_info.reward_total += *amount;
        pool_info_store(&mut deps.storage, *pool_type, n)
            .save(asset_token.as_str().as_bytes(), &pool_info)?;
    }

    Ok(HandleResponse {
        // send all the neb to the custody contract
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.nebula_token,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: cfg.custody,
                amount: rewards_amount,
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "deposit_reward"),
            log("amount", rewards_amount),
        ],
        data: None,
    })
}

// withdraw all rewards or single reward depending on asset_token
pub fn withdraw_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let cfg = read_config(&deps.storage)?;

    let reward_owner = env.message.sender;

    let mut contribution_tuples = vec![];

    for i in PoolType::ALL_TYPES.iter() {
        let contribution_bucket = contributions_read(&deps.storage, &reward_owner, **i);
        for kv in contribution_bucket.range(None, None, Order::Ascending) {
            let (k, _) = kv?;

            let asset_address = HumanAddr::from(unsafe { std::str::from_utf8_unchecked(&k) });
            contribution_tuples.push((i, asset_address));
        }
    }

    for (pool_type, asset_address) in contribution_tuples {
        contributions_to_pending_rewards(
            &mut deps.storage,
            &reward_owner,
            **pool_type,
            &asset_address,
        )?;
    }

    let reward_amt = read_pending_rewards(&deps.storage, &reward_owner);
    store_pending_rewards(&mut deps.storage, &reward_owner, Uint128::zero())?;

    let config: Config = read_config(&deps.storage)?;

    Ok(HandleResponse {
        messages: vec![
            // withdraw reward amount from custody contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.custody,
                msg: to_binary(&ExtHandleMsg::RequestNeb { amount: reward_amt })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.nebula_token,
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: reward_owner,
                    amount: reward_amt,
                })?,
                send: vec![],
            }),
        ],
        log: vec![
            log("action", "withdraw"),
            log("amount", reward_amt.to_string()),
        ],
        data: None,
    })
}

pub fn increment_n<S: Storage>(storage: &mut S) -> StdResult<()> {
    let current_n = read_current_n(storage)?;
    store_current_n(storage, current_n + 1)?;
    Ok(())
}
