use crate::querier::load_token_balance;
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_read, poll_store, poll_voter_read,
    poll_voter_store, read_bank_stakers, read_polls, state_read, state_store,
    total_voting_power_read, total_voting_power_store, Config, Poll, State, TokenManager,
    TotalVotingPower,
};

use cluster_math::FPDecimal;

use cosmwasm_std::{
    attr, to_binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Storage, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    PollStatus, SharesResponse, SharesResponseItem, StakerResponse, VoterInfo, VotingPowerResponse,
};

pub static SECONDS_PER_WEEK: u64 = 604800u64; //60 * 60 * 24 * 7
pub static M: u64 = 104u64; //Max weeks

pub fn adjust_total_voting_power(
    total_voting_power: &mut TotalVotingPower,
    current_week: u64,
    amount: u128,
    duration: u64,
    add: bool,
) -> StdResult<()> {
    // surely this is fine
    while total_voting_power.last_upd != current_week {
        total_voting_power.voting_power[(total_voting_power.last_upd % M) as usize] =
            FPDecimal::zero();
        total_voting_power.last_upd += 1;
    }

    for i in current_week..current_week + duration {
        let mut total =
            FPDecimal::from((current_week + duration - i) as u128) * FPDecimal::from(amount);
        total = total / FPDecimal::from(M as u128);
        let mut voting_power = total_voting_power.voting_power[(i % M) as usize];
        if add {
            voting_power = voting_power + FPDecimal::from(total);
        } else {
            voting_power = voting_power - FPDecimal::from(total);
        }
        total_voting_power.voting_power[(i % M) as usize] = voting_power;
    }

    Ok(())
}

pub fn stake_voting_tokens(
    deps: DepsMut,
    env: Env,
    sender: String,
    amount: Uint128,
    lock_for_weeks: Option<u64>,
) -> StdResult<Response> {
    if amount.is_zero() {
        return Err(StdError::generic_err("Insufficient funds sent"));
    }

    let key = &sender.as_str().as_bytes();

    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();
    let config: Config = config_store(deps.storage).load()?;
    let mut state: State = state_store(deps.storage).load()?;

    // balance already increased, so subtract deposit amount
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance =
        (load_token_balance(deps.as_ref(), &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance + amount))?;
    let share = if total_balance.is_zero() || state.total_share.is_zero() {
        amount
    } else {
        amount.multiply_ratio(state.total_share, total_balance)
    };

    let mut total_voting_power = total_voting_power_read(deps.storage).load()?;

    let current_week = env.block.time.seconds() / SECONDS_PER_WEEK;

    if let Some(_) = token_manager.lock_end_week {
        if lock_for_weeks.is_some() {
            return Err(StdError::generic_err("Cannot specify lock_for_weeks if tokens already staked. To change the lock time, use increase_lock_time"));
        }
        // remove existing impact of this address from voting pool
        adjust_total_voting_power(
            &mut total_voting_power,
            current_week,
            token_manager.share.u128(),
            token_manager.lock_end_week.unwrap() - current_week,
            false,
        )?;
    } else {
        if lock_for_weeks.is_none() {
            return Err(StdError::generic_err(
                "Must specify lock_for_weeks if no tokens staked.",
            ));
        }
        if lock_for_weeks.unwrap() > M {
            return Err(StdError::generic_err("Lock time exceeds the maximum."));
        }
        token_manager.lock_end_week = Some(current_week + lock_for_weeks.unwrap());
    }

    token_manager.share += share;
    state.total_share += share;

    // add impact of this address to voting pool
    adjust_total_voting_power(
        &mut total_voting_power,
        current_week,
        token_manager.share.u128(),
        token_manager.lock_end_week.unwrap() - current_week,
        true,
    )?;

    state_store(deps.storage).save(&state)?;
    bank_store(deps.storage).save(key, &token_manager)?;
    total_voting_power_store(deps.storage).save(&total_voting_power)?;
    let current_voting_power =
        calc_voting_power(share, token_manager.lock_end_week.unwrap(), current_week);

    Ok(Response::new().add_attributes(vec![
        attr("action", "staking"),
        attr("sender", sender.as_str()),
        attr("share", share.to_string()),
        attr("amount", amount.to_string()),
        attr("current_voting_power", current_voting_power.to_string()),
    ]))
}

// Withdraw amount if not staked. By default all funds will be withdrawn.
pub fn withdraw_voting_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> StdResult<Response> {
    let sender_address = info.sender.to_string();
    let key = sender_address.as_str().as_bytes();

    if let Some(mut token_manager) = bank_read(deps.storage).may_load(key)? {
        let config: Config = config_store(deps.storage).load()?;
        let mut state: State = state_store(deps.storage).load()?;

        // Load total share & total balance except proposal deposit amount
        let total_share = state.total_share.u128();
        let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
        let total_balance =
            (load_token_balance(deps.as_ref(), &config.nebula_token, &state.contract_addr)?
                .checked_sub(total_locked_balance))?
            .u128();

        let user_locked_balance = compute_locked_balance(deps.storage, &mut token_manager, &key)?;
        let user_locked_share = user_locked_balance * total_share / total_balance;
        let user_share = token_manager.share.u128();

        let withdraw_share = amount
            .map(|v| std::cmp::max(v.multiply_ratio(total_share, total_balance).u128(), 1u128))
            .unwrap_or_else(|| user_share - user_locked_share);
        let withdraw_amount = amount
            .map(|v| v.u128())
            .unwrap_or_else(|| withdraw_share * total_balance / total_share);

        if user_locked_share + withdraw_share > user_share {
            Err(StdError::generic_err(
                "User is trying to withdraw too many tokens.",
            ))
        } else if env.block.time.seconds() / SECONDS_PER_WEEK < token_manager.lock_end_week.unwrap()
        {
            //Check if locked time has passed before allowing
            Err(StdError::generic_err(
                "User is trying to withdraw tokens before expiry.",
            ))
        } else {
            let share = user_share - withdraw_share;
            token_manager.share = Uint128::from(share);

            bank_store(deps.storage).save(key, &token_manager)?;

            state.total_share = Uint128::from(total_share - withdraw_share);
            state_store(deps.storage).save(&state)?;

            send_tokens(
                &config.nebula_token,
                &sender_address,
                withdraw_amount,
                "withdraw",
            )
        }
    } else {
        Err(StdError::generic_err("Nothing staked"))
    }
}

// returns the largest locked amount in participated polls.
fn compute_locked_balance(
    storage: &mut dyn Storage,
    token_manager: &mut TokenManager,
    voter: &[u8],
) -> StdResult<u128> {
    // filter out not in-progress polls and get max locked
    let mut lock_entries_to_remove: Vec<u64> = vec![];

    let max_locked = token_manager
        .locked_balance
        .iter()
        .filter(|(poll_id, _)| {
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes()).unwrap();

            // cleanup not needed information, voting info in polls with no rewards
            if poll.status != PollStatus::InProgress && poll.voters_reward.is_zero() {
                poll_voter_store(storage, *poll_id).remove(voter);
                lock_entries_to_remove.push(*poll_id);
            }

            poll.status == PollStatus::InProgress
        })
        .map(|(_, v)| v.balance.u128())
        .max()
        .unwrap_or_default();

    // cleanup, check if there was any voter info removed
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !lock_entries_to_remove.contains(poll_id));

    Ok(max_locked)
}

pub fn deposit_reward(
    deps: DepsMut,
    _info: MessageInfo,
    _sender: String,
    amount: Uint128,
) -> StdResult<Response> {
    let config = config_read(deps.storage).load()?;

    let mut polls_in_progress = read_polls(
        deps.storage,
        Some(PollStatus::InProgress),
        None,
        None,
        None,
        Some(true), // remove hard cap to get all polls
    )?;

    if config.voter_weight.is_zero() || polls_in_progress.is_empty() {
        return Ok(Response::new().add_attributes(vec![
            attr("action", "deposit_reward"),
            attr("amount", amount.to_string()),
        ]));
    }

    let voter_rewards = amount * config.voter_weight;
    let rewards_per_poll =
        voter_rewards.multiply_ratio(Uint128::new(1), polls_in_progress.len() as u128);
    if rewards_per_poll.is_zero() {
        return Err(StdError::generic_err("Reward deposited is too small"));
    }
    for poll in polls_in_progress.iter_mut() {
        poll.voters_reward += rewards_per_poll;
        poll_store(deps.storage)
            .save(&poll.id.to_be_bytes(), &poll)
            .unwrap()
    }

    state_store(deps.storage).update(|mut state| -> StdResult<_> {
        state.pending_voting_rewards += voter_rewards;
        Ok(state)
    })?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "deposit_reward"),
        attr("amount", amount.to_string()),
    ]))
}

pub fn withdraw_voting_rewards(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let config: Config = config_store(deps.storage).load()?;
    let sender_address = info.sender.to_string();
    let key = sender_address.as_str().as_bytes();

    let mut token_manager = bank_read(deps.storage)
        .load(key)
        .or(Err(StdError::generic_err("Nothing staked")))?;

    let (user_reward_amount, w_polls) =
        withdraw_user_voting_rewards(deps.storage, &sender_address, &token_manager)?;
    if user_reward_amount.eq(&0u128) {
        return Err(StdError::generic_err("Nothing to withdraw"));
    }

    // cleanup, remove from locked_balance the polls from which we withdrew the rewards
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !w_polls.contains(poll_id));
    bank_store(deps.storage).save(key, &token_manager)?;

    state_store(deps.storage).update(|mut state| -> StdResult<_> {
        state.pending_voting_rewards = state
            .pending_voting_rewards
            .checked_sub(Uint128::new(user_reward_amount))?;
        Ok(state)
    })?;

    send_tokens(
        &config.nebula_token,
        &sender_address,
        user_reward_amount,
        "withdraw_voting_rewards",
    )
}

fn withdraw_user_voting_rewards(
    storage: &mut dyn Storage,
    user_address: &String,
    token_manager: &TokenManager,
) -> StdResult<(u128, Vec<u64>)> {
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(storage, token_manager, user_address);
    let user_reward_amount: u128 = w_polls
        .iter()
        .map(|(poll, voting_info)| {
            // remove voter info from the poll
            poll_voter_store(storage, poll.id).remove(user_address.as_str().as_bytes());

            // calculate reward share
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

fn get_withdrawable_polls(
    storage: &dyn Storage,
    token_manager: &TokenManager,
    user_address: &String,
) -> Vec<(Poll, VoterInfo)> {
    let w_polls: Vec<(Poll, VoterInfo)> = token_manager
        .locked_balance
        .iter()
        .map(|(poll_id, _)| {
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes()).unwrap();
            let voter_info_res: StdResult<VoterInfo> =
                poll_voter_read(storage, *poll_id).load(&user_address.as_str().as_bytes());
            (poll, voter_info_res)
        })
        .filter(|(poll, voter_info_res)| {
            poll.status != PollStatus::InProgress && voter_info_res.is_ok()
        })
        .map(|(poll, voter_info_res)| (poll, voter_info_res.unwrap()))
        .collect();
    w_polls
}

fn send_tokens(
    asset_token: &String,
    recipient: &String,
    amount: u128,
    action: &str,
) -> StdResult<Response> {
    let log = vec![
        attr("action", action),
        attr("recipient", recipient.as_str()),
        attr("amount", &amount.to_string()),
    ];

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::from(amount),
            })?,
            funds: vec![],
        })])
        .add_attributes(log))
}

pub fn query_staker(deps: Deps, address: String) -> StdResult<StakerResponse> {
    let config: Config = config_read(deps.storage).load()?;
    let state: State = state_read(deps.storage).load()?;
    let mut token_manager = bank_read(deps.storage)
        .may_load(address.as_str().as_bytes())?
        .unwrap_or_default();

    // calculate pending voting rewards
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(deps.storage, &token_manager, &address);
    let user_reward_amount: u128 = w_polls
        .iter()
        .map(|(poll, voting_info)| {
            // calculate reward share
            let total_votes =
                poll.no_votes.u128() + poll.yes_votes.u128() + poll.abstain_votes.u128();
            let poll_voting_reward = poll
                .voters_reward
                .multiply_ratio(voting_info.balance, total_votes);
            poll_voting_reward.u128()
        })
        .sum();

    // filter out not in-progress polls
    token_manager.locked_balance.retain(|(poll_id, _)| {
        let poll: Poll = poll_read(deps.storage)
            .load(&poll_id.to_be_bytes())
            .unwrap();

        poll.status == PollStatus::InProgress
    });

    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(deps, &config.nebula_token, &state.contract_addr)?
        .checked_sub(total_locked_balance))?;

    Ok(StakerResponse {
        balance: if !state.total_share.is_zero() {
            token_manager
                .share
                .multiply_ratio(total_balance, state.total_share)
        } else {
            Uint128::zero()
        },
        share: token_manager.share,
        locked_balance: token_manager.locked_balance,
        pending_voting_rewards: Uint128::new(user_reward_amount),
        lock_end_week: token_manager.lock_end_week,
    })
}

// Calculate current voting power of a user
pub fn calc_voting_power(share: Uint128, lock_end_week: u64, current_week: u64) -> FPDecimal {
    let locked_weeks_remaining = lock_end_week - current_week;
    let voting_power = FPDecimal::from(share.u128())
        * FPDecimal::from(locked_weeks_remaining as u128)
        / FPDecimal::from(M as u128);
    return voting_power;
}

pub fn query_total_voting_power(deps: Deps) -> StdResult<TotalVotingPower> {
    let total_power = total_voting_power_read(deps.storage).load()?;
    Ok(total_power)
}

pub fn query_voting_power(deps: Deps, env: Env, address: String) -> StdResult<VotingPowerResponse> {
    let state = state_read(deps.storage).load()?;
    let config = config_read(deps.storage).load()?;

    let key = &address.as_str().as_bytes();
    let token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    // convert share to amount
    let total_share = state.total_share;
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(deps, &config.nebula_token, &state.contract_addr)?
        .checked_sub(total_locked_balance))?;

    let voting_power = calc_voting_power(
        token_manager
            .share
            .multiply_ratio(total_balance, total_share),
        token_manager.lock_end_week.unwrap(),
        env.block.time.seconds() / SECONDS_PER_WEEK,
    );

    Ok(VotingPowerResponse {
        staker: address,
        voting_power: voting_power,
    })
}

//Increase the number of weeks staked tokens are locked for
pub fn increase_lock_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    increase_weeks: u64,
) -> StdResult<Response> {
    let sender_address = info.sender;
    let key = sender_address.as_str().as_bytes();

    let mut total_voting_power = total_voting_power_read(deps.storage).load()?;
    let current_week = env.block.time.seconds() / SECONDS_PER_WEEK;
    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    if let Some(lock_end_week) = token_manager.lock_end_week {
        if lock_end_week + increase_weeks - current_week > M {
            return Err(StdError::generic_err("Lock time exceeds the maximum."));
        }

        adjust_total_voting_power(
            &mut total_voting_power,
            current_week,
            token_manager.share.u128(),
            token_manager.lock_end_week.unwrap() - current_week,
            false,
        )?;

        token_manager.lock_end_week = Some(token_manager.lock_end_week.unwrap() + increase_weeks);

        adjust_total_voting_power(
            &mut total_voting_power,
            current_week,
            token_manager.share.u128(),
            token_manager.lock_end_week.unwrap() - current_week,
            true,
        )?;

        bank_store(deps.storage).save(key, &token_manager)?;
        total_voting_power_store(deps.storage).save(&total_voting_power)?;

        Ok(Response::new().add_attributes(vec![
            attr("action", "increase_lock_time"),
            attr("sender", sender_address.as_str()),
            attr("previous_lock_end_week", lock_end_week.to_string()),
            attr(
                "new_lock_end_week",
                (lock_end_week + increase_weeks).to_string(),
            ),
        ]))
    } else {
        Err(StdError::generic_err("User has no tokens staked."))
    }
}

pub fn query_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<SharesResponse> {
    let stakers: Vec<(String, TokenManager)> = if let Some(start_after) = start_after {
        read_bank_stakers(deps.storage, Some(start_after), limit, order_by)?
    } else {
        read_bank_stakers(deps.storage, None, limit, order_by)?
    };

    let stakers_shares: Vec<SharesResponseItem> = stakers
        .iter()
        .map(|item| {
            let (k, v) = item;
            SharesResponseItem {
                staker: k.clone(),
                share: v.share,
            }
        })
        .collect();

    Ok(SharesResponse {
        stakers: stakers_shares,
    })
}
