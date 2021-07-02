use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Decimal, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::rewards::before_share_change;
use crate::state::{
    read_pool_info, rewards_read, rewards_store, store_pool_info, PoolInfo, RewardInfo,
};

use cw20::Cw20HandleMsg;

pub fn bond<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    staker_addr: HumanAddr,
    asset_token: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    _increase_bond_amount(&mut deps.storage, &staker_addr, &asset_token, amount)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "bond"),
            log("staker_addr", staker_addr.as_str()),
            log("asset_token", asset_token.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn unbond<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    staker_addr: HumanAddr,
    asset_token: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let staking_token: HumanAddr =
        _decrease_bond_amount(&mut deps.storage, &staker_addr, &asset_token, amount)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_token,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: staker_addr.clone(),
                amount,
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "unbond"),
            log("staker_addr", staker_addr.as_str()),
            log("asset_token", asset_token.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

fn _increase_bond_amount<S: Storage>(
    storage: &mut S,
    staker_addr: &HumanAddr,
    asset_token: &HumanAddr,
    amount: Uint128,
) -> StdResult<()> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr)
        .load(asset_token.as_str().as_bytes())
        .unwrap_or_else(|_| RewardInfo {
            index: Decimal::zero(),
            bond_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
        });

    // Withdraw reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Increase total short or bond amount
    pool_info.total_bond_amount += amount;

    reward_info.bond_amount += amount;
    rewards_store(storage, &staker_addr).save(asset_token.as_str().as_bytes(), &reward_info)?;
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(())
}

fn _decrease_bond_amount<S: Storage>(
    storage: &mut S,
    staker_addr: &HumanAddr,
    asset_token: &HumanAddr,
    amount: Uint128,
) -> StdResult<HumanAddr> {
    let mut pool_info: PoolInfo = read_pool_info(storage, &asset_token)?;
    let mut reward_info: RewardInfo =
        rewards_read(storage, &staker_addr).load(asset_token.as_str().as_bytes())?;

    if reward_info.bond_amount < amount {
        return Err(StdError::generic_err("Cannot unbond more than bond amount"));
    }

    // Distribute reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Decrease total short or bond amount
    pool_info.total_bond_amount = (pool_info.total_bond_amount - amount)?;

    reward_info.bond_amount = (reward_info.bond_amount - amount)?;

    // Update rewards info
    if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
        rewards_store(storage, &staker_addr).remove(asset_token.as_str().as_bytes());
    } else {
        rewards_store(storage, &staker_addr).save(asset_token.as_str().as_bytes(), &reward_info)?;
    }

    // Update pool info
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(pool_info.staking_token)
}