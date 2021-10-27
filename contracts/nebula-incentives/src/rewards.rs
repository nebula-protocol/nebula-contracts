use cosmwasm_std::{
    attr, to_binary, CanonicalAddr, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdError,
    StdResult, Storage, Uint128, WasmMsg,
};

use crate::state::{
    contributions_read, contributions_to_pending_rewards, pool_info_read, pool_info_store,
    read_config, read_current_n, read_from_pool_bucket, read_pending_rewards, store_current_n,
    store_pending_rewards, Config, PoolInfo,
};

use nebula_protocol::incentives::{ExtExecuteMsg, PoolType};

use cw20::Cw20ExecuteMsg;

// anybody can deposit rewards...
pub fn deposit_reward(
    deps: DepsMut,
    // pool_type, asset_address, amount
    rewards: Vec<(u16, String, Uint128)>,
    rewards_amount: Uint128,
) -> StdResult<Response> {
    let cfg = read_config(deps.storage)?;
    let n = read_current_n(deps.storage)?;

    for (pool_type, asset_token, amount) in rewards.iter() {
        if !PoolType::ALL_TYPES.contains(&pool_type) {
            return Err(StdError::generic_err("pool type not found"));
        }
        let asset_token_raw = deps.api.addr_canonicalize(&asset_token)?;
        let mut pool_info: PoolInfo = read_from_pool_bucket(
            &pool_info_read(deps.storage, *pool_type, n),
            &asset_token_raw,
        );
        pool_info.reward_total += *amount;
        pool_info_store(deps.storage, *pool_type, n)
            .save(asset_token_raw.as_slice(), &pool_info)?;
    }

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&cfg.nebula_token)?.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: deps.api.addr_humanize(&cfg.custody)?.to_string(),
                amount: rewards_amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "deposit_reward"),
            attr("amount", rewards_amount),
        ]))
}

// withdraw all rewards or single reward depending on asset_token
pub fn withdraw_reward(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    let reward_owner = deps.api.addr_canonicalize(info.sender.as_str())?;

    let mut contribution_tuples = vec![];

    for i in PoolType::ALL_TYPES.iter() {
        let contribution_bucket = contributions_read(deps.storage, &reward_owner, **i);
        println!("contribution_bucket {:?}", contribution_tuples);
        for kv in contribution_bucket.range(None, None, Order::Ascending) {
            let (k, _) = kv?;

            let asset_address = deps
                .api
                .addr_humanize(&CanonicalAddr::from(k.clone()))?
                .to_string();
            contribution_tuples.push((i, asset_address));
        }
    }

    for (pool_type, asset_address) in contribution_tuples {
        let asset_address_raw = deps.api.addr_canonicalize(&asset_address)?;
        contributions_to_pending_rewards(
            deps.storage,
            &reward_owner,
            **pool_type,
            &asset_address_raw,
        )?;
    }

    let reward_amt = read_pending_rewards(deps.storage, &reward_owner);
    println!("reward amount: {:?}", reward_amt);
    store_pending_rewards(deps.storage, &reward_owner, Uint128::zero())?;

    Ok(Response::new()
        .add_messages(vec![
            // withdraw reward amount from custody contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.custody)?.to_string(),
                msg: to_binary(&ExtExecuteMsg::RequestNeb { amount: reward_amt })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.nebula_token)?.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: deps.api.addr_humanize(&reward_owner)?.to_string(),
                    amount: reward_amt,
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("amount", reward_amt.to_string()),
        ]))
}

pub fn increment_n(storage: &mut dyn Storage) -> StdResult<u64> {
    let current_n = read_current_n(storage)?;
    store_current_n(storage, current_n + 1)?;
    Ok(current_n + 1)
}
