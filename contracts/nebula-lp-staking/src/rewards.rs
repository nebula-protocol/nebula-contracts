use cosmwasm_std::{
    attr, to_binary, Addr, CosmosMsg, Decimal, Deps, DepsMut, MessageInfo, Order, Response,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::state::{
    read_config, read_pool_info, rewards_read, rewards_store, store_pool_info, Config, PoolInfo,
    RewardInfo,
};
use nebula_protocol::staking::{RewardInfoResponse, RewardInfoResponseItem};

use cw20::Cw20ExecuteMsg;

/// ## Description
/// Adds reward to LP staking pools.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **rewards** is an object of type [`Vec<(String, Uint128)>`] which is a list of rewards
///     deposited to each LP staking pool -- (cluster token, deposit amount).
///
/// - **rewards_amount** is an object of type [`Uint128`] which is the total deposit rewards.
pub fn deposit_reward(
    deps: DepsMut,
    rewards: Vec<(String, Uint128)>,
    rewards_amount: Uint128,
) -> Result<Response, ContractError> {
    for (asset_token, amount) in rewards.iter() {
        // Validate address format
        let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
        let mut pool_info: PoolInfo = read_pool_info(deps.storage, &validated_asset_token)?;
        let mut reward_amount = *amount;

        if pool_info.total_bond_amount.is_zero() {
            // If there is no bonding at all, cannot compute reward_per_bond
            // Store all deposit rewards to pending rewards
            pool_info.pending_reward += reward_amount;
        } else {
            // If there is some bonding, update the reward index

            // Take pending reward into account
            reward_amount += pool_info.pending_reward;
            pool_info.pending_reward = Uint128::zero();

            // Compute reward to be distribute per bond for this round
            let normal_reward_per_bond =
                Decimal::from_ratio(reward_amount, pool_info.total_bond_amount);

            // Update the reward index
            // -- new_reward_index = old_reward_index + reward_per_bond_this_round
            pool_info.reward_index = pool_info.reward_index + normal_reward_per_bond;
        }

        store_pool_info(deps.storage, &validated_asset_token, &pool_info)?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "deposit_reward"),
        attr("rewards_amount", rewards_amount.to_string()),
    ]))
}

/// ## Description
/// Withdraws all rewards or single reward depending on `asset_token`.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_token** is an object of type [`Option<String>`] which is an address of
///     a cluster token contract.
pub fn withdraw_reward(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: Option<String>,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_asset_token = asset_token
        .map(|x| deps.api.addr_validate(x.as_str()))
        .transpose()?;
    let staker_addr = info.sender;
    // Compute the pending rewards of the staker
    let reward_amount = _withdraw_reward(deps.storage, &staker_addr, &validated_asset_token)?;

    // Transfer rewards from this LP staking contract to the message sender / staker
    let config: Config = read_config(deps.storage)?;
    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: staker_addr.to_string(),
                amount: reward_amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("amount", reward_amount.to_string()),
        ]))
}

/// ## Description
/// Computes all pending rewards or single pending reward of a staker depending on `asset_token`.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **staker_addr** is a reference to an object of type [`Addr`] which is a staker address.
///
/// - **asset_token** is a reference to an object of type [`Option<Addr>`] which is an address
///     of a cluster token contract.
fn _withdraw_reward(
    storage: &mut dyn Storage,
    staker_addr: &Addr,
    asset_token: &Option<Addr>,
) -> Result<Uint128, ContractError> {
    // Get all rewards owned by this staker
    let rewards_bucket = rewards_read(storage, staker_addr);

    let reward_pairs: Vec<(Addr, RewardInfo)>;
    if let Some(asset_token) = asset_token {
        // Withdraw single reward

        let reward_info = rewards_bucket.may_load(asset_token.as_bytes())?;
        reward_pairs = if let Some(reward_info) = reward_info {
            vec![(asset_token.clone(), reward_info)]
        } else {
            vec![]
        };
    } else {
        // Withdraw all rewards
        reward_pairs =
            rewards_bucket
                .range(None, None, Order::Ascending)
                .map(|item| {
                    let (k, v) = item?;
                    Ok((
                        Addr::unchecked(std::str::from_utf8(&k).map_err(|_| {
                            ContractError::Invalid("reward pair address".to_string())
                        })?),
                        v,
                    ))
                })
                .collect::<Result<Vec<(Addr, RewardInfo)>, ContractError>>()?;
    }

    let mut amount: Uint128 = Uint128::zero();
    for reward_pair in reward_pairs {
        let (asset_token, mut reward_info) = reward_pair;
        let pool_info: PoolInfo = read_pool_info(storage, &asset_token)?;

        // Withdraw reward to staker pending reward
        before_share_change(&pool_info, &mut reward_info)?;
        amount += reward_info.pending_reward;
        reward_info.pending_reward = Uint128::zero();

        // Update rewards info
        if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
            rewards_store(storage, staker_addr).remove(asset_token.as_bytes());
        } else {
            rewards_store(storage, staker_addr).save(asset_token.as_bytes(), &reward_info)?;
        }
    }

    Ok(amount)
}

/// ## Description
/// Withdraws current reward to staker's pending reward
///
/// ## Params
/// - **pool_info** is a reference to an object of type [`PoolInfo`] which is the information of
///     a LP staking pool.
///
/// - **reward_info** is a mutable reference to an object of type [`RewardInfo`] which is
///     the staker related information to the LP staking pool.
#[allow(clippy::suspicious_operation_groupings)]
pub fn before_share_change(pool_info: &PoolInfo, reward_info: &mut RewardInfo) -> StdResult<()> {
    // Calculate the current pending rewards
    // -- pending rewards = staker_bond * (pool_reward_index - staker_reward_index)
    //                    = staker_bonding * (reward_per_bond_i + ... + reward_per_bond_j)
    let pending_reward = (reward_info.bond_amount * pool_info.reward_index)
        .checked_sub(reward_info.bond_amount * reward_info.index)?;

    // Update staker reward index and add pending reward
    reward_info.index = pool_info.reward_index;
    reward_info.pending_reward += pending_reward;
    Ok(())
}

/// ## Description
/// Returns staker reward information on a specific LP staking pool. Return all rewards if
/// `asset_token` is not specified.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **staker_addr** is an object of type [`String`] which is the staker address.
///
/// - **asset_token** is an object of type [`Option<String>`] which is an address of
///     a cluster token contract.
pub fn query_reward_info(
    deps: Deps,
    staker_addr: String,
    asset_token: Option<String>,
) -> StdResult<RewardInfoResponse> {
    // Validate address format
    let validated_staker_addr = deps.api.addr_validate(staker_addr.as_str())?;
    let validated_asset_token = asset_token
        .map(|x| deps.api.addr_validate(x.as_str()))
        .transpose()?;

    // Retrieve the reward information of the staker on the CT related LP staking pool
    let reward_infos: Vec<RewardInfoResponseItem> =
        _read_reward_infos(deps.storage, &validated_staker_addr, &validated_asset_token)?;

    Ok(RewardInfoResponse {
        staker_addr,
        reward_infos,
    })
}

/// ## Description
/// Returns all rewards or single reward of a staker depending on `asset_token` as a vector
/// of custom struct [`RewardInfoResponseItem`].
///
/// ## Params
/// - **storage** is a reference to an object implementing trait [`Storage`].
///
/// - **staker_addr** is a reference to an object of type [`Addr`] which is the staker address.
///
/// - **asset_token** is a reference to an object of type [`Option<Addr>`] which is an address
///     of a cluster token contract.
fn _read_reward_infos(
    storage: &dyn Storage,
    staker_addr: &Addr,
    asset_token: &Option<Addr>,
) -> StdResult<Vec<RewardInfoResponseItem>> {
    // Get all rewards owned by this staker
    let rewards_bucket = rewards_read(storage, staker_addr);
    let reward_infos: Vec<RewardInfoResponseItem>;
    if let Some(asset_token) = asset_token {
        // Read single reward
        reward_infos =
            if let Some(mut reward_info) = rewards_bucket.may_load(asset_token.as_bytes())? {
                // Get LP staking pool information
                let pool_info = read_pool_info(storage, asset_token)?;
                // Add newer rewards to pending rewards
                before_share_change(&pool_info, &mut reward_info)?;

                vec![RewardInfoResponseItem {
                    asset_token: asset_token.to_string(),
                    bond_amount: reward_info.bond_amount,
                    pending_reward: reward_info.pending_reward,
                }]
            } else {
                vec![]
            };
    } else {
        // Read all rewards
        reward_infos = rewards_bucket
            .range(None, None, Order::Ascending)
            .map(|item| {
                let (k, v) = item?;
                let asset_token = Addr::unchecked(
                    std::str::from_utf8(&k)
                        .map_err(|_| StdError::invalid_utf8("invalid asset token address"))?,
                );
                let mut reward_info = v;
                // Get LP staking pool information
                let pool_info = read_pool_info(storage, &asset_token)?;
                // Add newer rewards to pending rewards
                before_share_change(&pool_info, &mut reward_info)?;

                Ok(RewardInfoResponseItem {
                    asset_token: asset_token.to_string(),
                    bond_amount: reward_info.bond_amount,
                    pending_reward: reward_info.pending_reward,
                })
            })
            .collect::<StdResult<Vec<RewardInfoResponseItem>>>()?;
    }

    Ok(reward_infos)
}
