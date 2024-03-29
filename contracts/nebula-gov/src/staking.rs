use crate::error::ContractError;
use crate::querier::load_token_balance;
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_read, poll_store, poll_voter_read,
    poll_voter_store, read_bank_stakers, read_polls, state_read, state_store, Config, Poll, State,
    TokenManager,
};

use cosmwasm_std::{
    attr, to_binary, Addr, CosmosMsg, Deps, DepsMut, MessageInfo, Response, StdResult, Storage,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    PollStatus, SharesResponse, SharesResponseItem, StakerResponse, VoterInfo,
};

/// ## Description
/// Stakes the transferred amount into the governance contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **sender** is an object of type [`String`] which is the address of the
///     token sender / staker.
///
/// - **amount** is an object of type [`Uint128`] which is the Nebula amount to stake.
pub fn stake_voting_tokens(
    deps: DepsMut,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::Generic(
            "Insufficient funds sent".to_string(),
        ));
    }

    // Validate address format
    let sender_address = deps.api.addr_validate(sender.as_str())?;
    let key = sender_address.as_bytes();

    // Get the token manager of the sender
    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();
    let config: Config = config_store(deps.storage).load()?;
    let mut state: State = state_store(deps.storage).load()?;

    // Compute the total stake without the new stake `amount`
    // -- Governance total Nebula balance = total stake + total deposit + all voting rewards
    //                                    = (prev total stake + `amount`) + total locked balance
    //
    // -- Prev total stake = governance total Nebula balance - (total locked balance + `amount`)
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance =
        load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance + amount)?;

    // Compute the additional share from this staking amount
    // -- share = stake * total_share / total_stake
    let share = if total_balance.is_zero() || state.total_share.is_zero() {
        amount
    } else {
        amount.multiply_ratio(state.total_share, total_balance)
    };

    // Update the staker share and the total share
    token_manager.share += share;
    state.total_share += share;

    state_store(deps.storage).save(&state)?;
    bank_store(deps.storage).save(key, &token_manager)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "staking"),
        attr("sender", sender.as_str()),
        attr("share", share.to_string()),
        attr("amount", amount.to_string()),
    ]))
}

/// ## Description
/// Withdraws amount if not locked, votes + poll deposits.
/// By default all stake of the sender will be withdrawn.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **amount** is an object of type [`Option<Uint128>`] which is the amount to be withdrawn if specified.
pub fn withdraw_voting_tokens(
    deps: DepsMut,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let sender_address = info.sender;
    let key = sender_address.as_bytes();

    if let Some(mut token_manager) = bank_read(deps.storage).may_load(key)? {
        let config: Config = config_store(deps.storage).load()?;
        let mut state: State = state_store(deps.storage).load()?;

        // Load total share & total staking balance
        // Governance total Nebula balance = total stake + total deposit + all voting rewards
        let total_share = state.total_share.u128();
        let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
        let total_balance =
            (load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
                .checked_sub(total_locked_balance))?
            .u128();

        // Compute the sender locked balance, max vote amount + all proposal deposit
        let user_locked_balance =
            compute_locked_balance(deps.storage, &mut token_manager, &sender_address)?;
        // Compute the sender locked share
        // -- locked share = locked balance * total_share / total_stake
        let user_locked_share = user_locked_balance * total_share / total_balance;
        let user_share = token_manager.share.u128();

        // Compute the withdrawn share
        // If `amount` is provided,
        //      withdrawn share = withdrawn_amount * total_share / total_stake
        // Otherwise, withdrawing all non-locked share of the sender
        //      withdrawn share = user_total_share - user_locked_share
        let withdraw_share = amount
            .map(|v| std::cmp::max(v.multiply_ratio(total_share, total_balance).u128(), 1u128))
            .unwrap_or_else(|| user_share - user_locked_share);
        // Get the withdrawn amount. If `amount` is not provided, calculate from withdrawn share instead.
        let withdraw_amount = amount
            .map(|v| v.u128())
            .unwrap_or_else(|| withdraw_share * total_balance / total_share);

        if user_locked_share + withdraw_share > user_share {
            // Sender does not have enough share to be withdrawn
            Err(ContractError::Generic(
                "User is trying to withdraw too many tokens".to_string(),
            ))
        } else {
            // Decrease and update the user share
            let share = user_share - withdraw_share;
            token_manager.share = Uint128::from(share);

            bank_store(deps.storage).save(key, &token_manager)?;

            // Decrease and update the total share
            state.total_share = Uint128::from(total_share - withdraw_share);
            state_store(deps.storage).save(&state)?;

            // Send the withdrawn amount to the sender
            send_tokens(
                &config.nebula_token,
                &sender_address,
                withdraw_amount,
                "withdraw",
            )
        }
    } else {
        Err(ContractError::NothingStaked {})
    }
}

/// ## Description
/// Returns the largest locked amount in participated polls.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **token_manager** is a mutable reference to an object of type [`TokenManager`] which is an governance
///     related information of the withdrawing account.
///
/// - **voter** is a reference to an object of type [`Addr`] which is the voter address.
fn compute_locked_balance(
    storage: &mut dyn Storage,
    token_manager: &mut TokenManager,
    voter: &Addr,
) -> Result<u128, ContractError> {
    // Filter out not in-progress polls and get max locked
    let mut lock_entries_to_remove: Vec<u64> = vec![];
    let max_locked = token_manager
        .locked_balance
        .iter()
        .filter(|(poll_id, _)| {
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes()).unwrap();

            // Cleanup not needed information, voting info in polls with no rewards
            if poll.status != PollStatus::InProgress && poll.voters_reward.is_zero() {
                poll_voter_store(storage, *poll_id).remove(voter.as_bytes());
                lock_entries_to_remove.push(*poll_id);
            }

            poll.status == PollStatus::InProgress
        })
        .map(|(_, v)| v.balance.u128())
        .max()
        .unwrap_or_default();

    // Cleanup, remove voting infos of non in-progress polls with no rewards
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !lock_entries_to_remove.contains(poll_id));

    Ok(max_locked)
}

/// ## Description
/// Splits the deposited rewards to Nebula stakers and proposal voters based on `voter_weight` in
/// the configuration.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **amount** is an object of type [`Uint128`] which is the amount of rewards being deposited.
pub fn deposit_reward(deps: DepsMut, amount: Uint128) -> Result<Response, ContractError> {
    let config = config_read(deps.storage).load()?;

    // Retrieve all in-progress polls
    let mut polls_in_progress = read_polls(
        deps.storage,
        Some(PollStatus::InProgress),
        None,
        None,
        None,
        Some(true), // remove hard cap to get all polls
    )?;

    // If `voter_weight` is 0 or no polls in progress, do nothing
    if config.voter_weight.is_zero() || polls_in_progress.is_empty() {
        return Ok(Response::new().add_attributes(vec![
            attr("action", "deposit_reward"),
            attr("amount", amount.to_string()),
        ]));
    }

    // Calculate voter rewards per poll
    let voter_rewards = amount * config.voter_weight;
    let rewards_per_poll =
        voter_rewards.multiply_ratio(Uint128::new(1), polls_in_progress.len() as u128);
    if rewards_per_poll.is_zero() {
        return Err(ContractError::Generic(
            "Reward deposited is too small".to_string(),
        ));
    }
    // Increase voter rewards of each in-progress poll
    for poll in polls_in_progress.iter_mut() {
        poll.voters_reward += rewards_per_poll;
        poll_store(deps.storage)
            .save(&poll.id.to_be_bytes(), poll)
            .unwrap()
    }

    // Update total pending voting rewards
    state_store(deps.storage).update(|mut state| -> StdResult<_> {
        state.pending_voting_rewards += voter_rewards;
        Ok(state)
    })?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "deposit_reward"),
        attr("amount", amount.to_string()),
    ]))
}

/// ## Description
/// Withdraws voting rewards from all proposals or one specific proposal.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **poll_id** is an object of type [`Option<u64>`] which is the poll ID to withdraw a voting reward from.
pub fn withdraw_voting_rewards(
    deps: DepsMut,
    info: MessageInfo,
    poll_id: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = config_store(deps.storage).load()?;
    let sender_address = info.sender;
    let key = sender_address.as_bytes();

    // Get the token manager of the sender
    let mut token_manager = bank_read(deps.storage)
        .load(key)
        .map_err(|_| ContractError::NothingStaked {})?;

    // Get the total rewards to be withdrawn, and the polls from which rewards are withdrawn
    let (user_reward_amount, w_polls) =
        withdraw_user_voting_rewards(deps.storage, &sender_address, &token_manager, poll_id)?;
    if user_reward_amount.eq(&0u128) {
        return Err(ContractError::NothingToWithdraw {});
    }

    // Cleanup, remove the polls from which we withdrew the rewards from the sender locked balance
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !w_polls.contains(poll_id));
    bank_store(deps.storage).save(key, &token_manager)?;

    // Decrease and update the total pending rewards in the contract
    state_store(deps.storage).update(|mut state| -> StdResult<_> {
        state.pending_voting_rewards = state
            .pending_voting_rewards
            .checked_sub(Uint128::new(user_reward_amount))?;
        Ok(state)
    })?;

    // Send the withdrawn rewards to the sender
    send_tokens(
        &config.nebula_token,
        &sender_address,
        user_reward_amount,
        "withdraw_voting_rewards",
    )
}

/// ## Description
/// Stakes voting rewards back to governance from all proposals or one specific proposal.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **poll_id** is an object of type [`Option<u64>`] which is the poll ID to stake a voting reward.
pub fn stake_voting_rewards(
    deps: DepsMut,
    info: MessageInfo,
    poll_id: Option<u64>,
) -> Result<Response, ContractError> {
    let config: Config = config_store(deps.storage).load()?;
    let mut state: State = state_store(deps.storage).load()?;
    let sender_address = info.sender;
    let key = sender_address.as_bytes();

    // Get the token manager of the sender
    let mut token_manager = bank_read(deps.storage)
        .load(key)
        .map_err(|_| ContractError::NothingStaked {})?;

    // Get the total rewards to be withdrawn, and the polls from which rewards are withdrawn
    let (user_reward_amount, w_polls) =
        withdraw_user_voting_rewards(deps.storage, &sender_address, &token_manager, poll_id)?;
    if user_reward_amount.eq(&0u128) {
        return Err(ContractError::NothingToWithdraw {});
    }

    // Compute the current total stake
    // Governance total Nebula balance = total stake + total deposit + all voting rewards
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance =
        load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance)?;

    // Decrease the total pending rewards in the contract
    state.pending_voting_rewards = state
        .pending_voting_rewards
        .checked_sub(Uint128::new(user_reward_amount))?;

    // Compute the additional share from staking the rewards
    // -- share = rewards * total_share / total_stake
    let share: Uint128 = if total_balance.is_zero() || state.total_share.is_zero() {
        Uint128::new(user_reward_amount)
    } else {
        Uint128::new(user_reward_amount).multiply_ratio(state.total_share, total_balance)
    };

    // Update the staker share and the total share
    token_manager.share += share;
    state.total_share += share;

    // Cleanup, remove the polls from which we withdrew the rewards from the sender locked balance
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !w_polls.contains(poll_id));

    // Update governance state and the account data
    state_store(deps.storage).save(&state)?;
    bank_store(deps.storage).save(key, &token_manager)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "stake_voting_rewards"),
        attr("staker", sender_address.to_string()),
        attr("share", share.to_string()),
        attr("amount", user_reward_amount.to_string()),
    ]))
}

/// ## Description
/// Computes the pending voting rewards of an account from all proposals or one if specified.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **user_address** is a reference to an object of type [`Addr`] which is an address of the
///     withdrawing account.
///
/// - **token_manager** is a reference to an object of type [`TokenManager`] which is an governance
///     related information of the withdrawing account.
///
/// - **poll_id** is an object of type [`Option<u64>`] which is the poll ID to withdraw a voting reward from.
fn withdraw_user_voting_rewards(
    storage: &mut dyn Storage,
    user_address: &Addr,
    token_manager: &TokenManager,
    poll_id: Option<u64>,
) -> Result<(u128, Vec<u64>), ContractError> {
    let w_polls: Vec<(Poll, VoterInfo)> = match poll_id {
        // If `poll_id` is specified, get the user's vote info in that poll
        Some(poll_id) => {
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes())?;
            let voter_info = poll_voter_read(storage, poll_id).load(user_address.as_bytes())?;
            if poll.status == PollStatus::InProgress {
                return Err(ContractError::Generic(
                    "This poll is still in progress".to_string(),
                ));
            }
            if poll.voters_reward.is_zero() {
                return Err(ContractError::Generic(
                    "This poll has no voting rewards".to_string(),
                ));
            }
            vec![(poll, voter_info)]
        }
        // Otherwise, get the user's vote infos from all reward withdrawable polls
        None => get_withdrawable_polls(storage, token_manager, user_address),
    };
    // Calculate the total rewards from each poll vote
    let user_reward_amount: u128 = w_polls
        .iter()
        .map(|(poll, voting_info)| {
            // Remove voter info from the poll
            poll_voter_store(storage, poll.id).remove(user_address.as_bytes());

            // Calculate the user reward portion in this poll
            // -- poll voting reward = poll_vote_amount * poll_rewards / total_votes
            let total_votes =
                poll.no_votes.u128() + poll.yes_votes.u128() + poll.abstain_votes.u128();
            let poll_voting_reward = poll
                .voters_reward
                .multiply_ratio(voting_info.balance, total_votes);
            poll_voting_reward.u128()
        })
        .sum();
    Ok((
        user_reward_amount,
        w_polls.iter().map(|(poll, _)| poll.id).collect(),
    ))
}

/// ## Description
/// Returns all proposals and corresponding vote info from which the user can withdraw rewards.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **token_manager** is a reference to an object of type [`TokenManager`] which is an governance
///     related information of the withdrawing account.
///
/// - **user_address** is a reference to an object of type [`Addr`] which is an address of the
///     withdrawing account.
fn get_withdrawable_polls(
    storage: &dyn Storage,
    token_manager: &TokenManager,
    user_address: &Addr,
) -> Vec<(Poll, VoterInfo)> {
    let w_polls: Vec<(Poll, VoterInfo)> = token_manager
        .locked_balance
        .iter()
        .map(|(poll_id, _)| {
            // Get the poll info and vote info
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes()).unwrap();
            let voter_info_res: StdResult<VoterInfo> =
                poll_voter_read(storage, *poll_id).load(user_address.as_bytes());
            (poll, voter_info_res)
        })
        .filter(|(poll, voter_info_res)| {
            // Filter only non in-progress polls and have rewards
            poll.status != PollStatus::InProgress
                && voter_info_res.is_ok()
                && !poll.voters_reward.is_zero()
        })
        .map(|(poll, voter_info_res)| (poll, voter_info_res.unwrap()))
        .collect();
    w_polls
}

/// ## Description
/// Sends `asset_token` of the specified `amount` to the recipient.
///
/// ## Params
/// - **asset_token** is a reference to an object of type [`Addr`] which supposedly is the
///     address of Nebula token contract.
///
/// - **recipient** is a reference to an object of type [`Addr`] which is an address of
///     the recipient account.
///
/// - **amount** is an object of type [`u128`] which is the amount to transfer.
///
/// - **action** is a reference to an object of type [`str`] which describes the purpose
///     of this transfer.
fn send_tokens(
    asset_token: &Addr,
    recipient: &Addr,
    amount: u128,
    action: &str,
) -> Result<Response, ContractError> {
    let attributes = vec![
        attr("action", action),
        attr("recipient", recipient.to_string().as_str()),
        attr("amount", &amount.to_string()),
    ];

    // Transfer the asset
    let r = Response::new()
        .add_attributes(attributes)
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::from(amount),
            })?,
            funds: vec![],
        }));
    Ok(r)
}

/// ## Description
/// Returns the specified staker information.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **address** ia an object of type [`String`] which is an address of a staker to be queried.
pub fn query_staker(deps: Deps, address: String) -> StdResult<StakerResponse> {
    let addr_raw = deps.api.addr_validate(address.as_str())?;
    let config: Config = config_read(deps.storage).load()?;
    let state: State = state_read(deps.storage).load()?;
    let mut token_manager = bank_read(deps.storage)
        .may_load(addr_raw.as_bytes())?
        .unwrap_or_default();

    // Retrieve all withdrawable rewards
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(deps.storage, &token_manager, &addr_raw);

    // Calculate pending voting rewards
    let mut user_reward_amount = Uint128::zero();
    let w_polls_res: Vec<(u64, Uint128)> = w_polls
        .iter()
        .map(|(poll, voting_info)| {
            // Calculate the user reward portion in this poll
            // -- poll voting reward = poll_vote_amount * poll_rewards / total_votes
            let total_votes = poll.no_votes + poll.yes_votes + poll.abstain_votes;
            let poll_voting_reward = poll
                .voters_reward
                .multiply_ratio(voting_info.balance, total_votes);
            // Aggregate all the rewards
            user_reward_amount += poll_voting_reward;

            (poll.id, poll_voting_reward)
        })
        .collect();

    // Filter out not in-progress polls
    token_manager.locked_balance.retain(|(poll_id, _)| {
        let poll: Poll = poll_read(deps.storage)
            .load(&poll_id.to_be_bytes())
            .unwrap();

        poll.status == PollStatus::InProgress
    });

    // Compute the current total stake
    // Governance total Nebula balance = total stake + total deposit + all voting rewards
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance =
        load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance)?;

    Ok(StakerResponse {
        // balance / stake = share * total_stakes / total_share
        //                 = actual staked amount + staking rewards
        balance: if !state.total_share.is_zero() {
            token_manager
                .share
                .multiply_ratio(total_balance, state.total_share)
        } else {
            Uint128::zero()
        },
        share: token_manager.share,
        locked_balance: token_manager.locked_balance,
        pending_voting_rewards: user_reward_amount,
        withdrawable_polls: w_polls_res,
    })
}

/// ## Description
/// Returns a list of staker shares filtered by the provided criterions.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **start_after** is an object of type [`Option<String>`] which is a filter for staker address.
///
/// - **limit** is an object of type [`Option<u32>`] which limits the number of stakers in the query result.
///
/// - **order_by** is an object of type [`Option<OrderBy>`] which specifies the ordering of the result.
pub fn query_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<SharesResponse> {
    // Query a list of stakers and token managers based on the provided criterions
    let stakers: Vec<(Addr, TokenManager)> = if let Some(start_after) = start_after {
        read_bank_stakers(
            deps.storage,
            Some(deps.api.addr_validate(start_after.as_str())?),
            limit,
            order_by,
        )?
    } else {
        read_bank_stakers(deps.storage, None, limit, order_by)?
    };

    // Extract the staker's address and share
    let stakers_shares: Vec<SharesResponseItem> = stakers
        .iter()
        .map(|item| {
            let (k, v) = item;
            SharesResponseItem {
                staker: k.to_string(),
                share: v.share,
            }
        })
        .collect();

    Ok(SharesResponse {
        stakers: stakers_shares,
    })
}
