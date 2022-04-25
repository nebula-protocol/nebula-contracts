use cosmwasm_std::{
    attr, to_binary, CosmosMsg, DepsMut, MessageInfo, Order, Response, StdResult, Storage, Uint128,
    WasmMsg,
};

use crate::error::ContractError;
use crate::state::{
    contributions_read, contributions_to_pending_rewards, pool_info_read, pool_info_store,
    read_config, read_current_n, read_from_pool_bucket, read_pending_rewards, store_current_n,
    store_pending_rewards, Config, PoolInfo,
};

use nebula_protocol::proxy::{ExtExecuteMsg, PoolType};

use cw20::Cw20ExecuteMsg;

/// ## Description
/// Add rewards to pools. Note that anybody can deposit rewards.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **rewards** is an object of type [`Vec<(u16, String, Uint128)>`] which is a list of rewards to be deposited.
///     Each tuple contains (pool_type, cluster_contract, amount).
///
/// - **rewards_amount** is an object of type [`Uint128`] which is the sum of the rewards in the list.
pub fn deposit_reward(
    deps: DepsMut,
    rewards: Vec<(u16, String, Uint128)>,
    rewards_amount: Uint128,
) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;
    let n = read_current_n(deps.storage)?;

    for (pool_type, cluster_contract, amount) in rewards.iter() {
        // Validate address format
        let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;
        // Check the pool type
        if !PoolType::ALL_TYPES.contains(&pool_type) {
            return Err(ContractError::Generic("Pool type not found".to_string()));
        }
        // Update the rewards in the pool
        let mut pool_info: PoolInfo = read_from_pool_bucket(
            &pool_info_read(deps.storage, *pool_type, n),
            &validated_cluster_contract,
        );
        pool_info.reward_total += *amount;
        pool_info_store(deps.storage, *pool_type, n)
            .save(validated_cluster_contract.as_bytes(), &pool_info)?;
    }

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.nebula_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: cfg.custody.to_string(),
                amount: rewards_amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "deposit_reward"),
            attr("amount", rewards_amount),
        ]))
}

/// ## Description
/// Withdraws all rewards for the sender.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
pub fn withdraw_reward(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let cfg = read_config(deps.storage)?;

    let reward_owner = info.sender;

    let mut contribution_tuples = vec![];

    for i in PoolType::ALL_TYPES.iter() {
        let contribution_bucket = contributions_read(deps.storage, &reward_owner, **i);
        for kv in contribution_bucket.range(None, None, Order::Ascending) {
            let (k, _) = kv?;

            let asset_address = deps.api.addr_validate(
                std::str::from_utf8(&k)
                    .map_err(|_| ContractError::Invalid("asset address".to_string()))?,
            )?;
            contribution_tuples.push((i, asset_address));
        }
    }

    // Compute and aggregate user pending rewards
    for (pool_type, asset_address) in contribution_tuples {
        contributions_to_pending_rewards(deps.storage, &reward_owner, **pool_type, &asset_address)?;
    }

    // Retrieve the total user pending rewards, and reset the pending rewards
    let reward_amt = read_pending_rewards(deps.storage, &reward_owner);
    store_pending_rewards(deps.storage, &reward_owner, Uint128::zero())?;

    let config: Config = read_config(deps.storage)?;

    Ok(Response::new()
        .add_messages(vec![
            // Withdraw reward amount from custody contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.custody.to_string(),
                msg: to_binary(&ExtExecuteMsg::RequestNeb { amount: reward_amt })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.nebula_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: reward_owner.to_string(),
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

/// ## Description
/// Increases the current penalty period by one.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
pub fn increment_n(storage: &mut dyn Storage) -> StdResult<u64> {
    let current_n = read_current_n(storage)?;
    store_current_n(storage, current_n + 1)?;
    Ok(current_n + 1)
}
