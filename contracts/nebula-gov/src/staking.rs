use crate::querier::load_token_balance;
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_read, poll_store, poll_voter_read,
    poll_voter_store, read_bank_stakers, read_polls, state_read, state_store,
    total_voting_power_read, total_voting_power_store, Config, Poll, State, TokenManager,
    TotalVotingPower,
};

use cluster_math::FPDecimal;

use cosmwasm_std::{
    log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    PollStatus, SharesResponse, SharesResponseItem, StakerResponse, VoterInfo,
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

pub fn stake_voting_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    amount: Uint128,
    lock_for_weeks: Option<u64>,
) -> HandleResult {
    if amount.is_zero() {
        return Err(StdError::generic_err("Insufficient funds sent"));
    }

    let key = &sender.as_str().as_bytes();

    let mut token_manager = bank_read(&deps.storage).may_load(key)?.unwrap_or_default();
    let config: Config = config_store(&mut deps.storage).load()?;
    let mut state: State = state_store(&mut deps.storage).load()?;

    // balance already increased, so subtract deposit amount
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(&deps, &config.nebula_token, &state.contract_addr)?
        - (total_locked_balance + amount))?;
    let share = if total_balance.is_zero() || state.total_share.is_zero() {
        amount
    } else {
        amount.multiply_ratio(state.total_share, total_balance)
    };

    let mut total_voting_power = total_voting_power_read(&deps.storage).load()?;

    let current_week = env.block.time / SECONDS_PER_WEEK;

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

    state_store(&mut deps.storage).save(&state)?;
    bank_store(&mut deps.storage).save(key, &token_manager)?;
    total_voting_power_store(&mut deps.storage).save(&total_voting_power)?;

    Ok(HandleResponse {
        messages: vec![],
        data: None,
        log: vec![
            log("action", "staking"),
            log("sender", sender.as_str()),
            log("share", share.to_string()),
            log("amount", amount.to_string()),
        ],
    })
}

// Withdraw amount if not staked. By default all funds will be withdrawn.
pub fn withdraw_voting_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Option<Uint128>,
) -> HandleResult {
    let sender_address = env.message.sender;
    let key = sender_address.as_str().as_bytes();

    if let Some(mut token_manager) = bank_read(&deps.storage).may_load(key)? {
        let config: Config = config_store(&mut deps.storage).load()?;
        let mut state: State = state_store(&mut deps.storage).load()?;

        // Load total share & total balance except proposal deposit amount
        let total_share = state.total_share.u128();
        let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
        let total_balance =
            (load_token_balance(&deps, &config.nebula_token, &state.contract_addr)?
                - total_locked_balance)?
                .u128();

        let user_locked_balance = compute_locked_balance(deps, &mut token_manager, &key)?;
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
        } else if env.block.time / SECONDS_PER_WEEK < token_manager.lock_end_week.unwrap() {
            //Check if locked time has passed before allowing
            Err(StdError::generic_err(
                "User is trying to withdraw tokens before expiry.",
            ))
        } else {
            let share = user_share - withdraw_share;
            token_manager.share = Uint128::from(share);

            bank_store(&mut deps.storage).save(key, &token_manager)?;

            state.total_share = Uint128::from(total_share - withdraw_share);
            state_store(&mut deps.storage).save(&state)?;

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
fn compute_locked_balance<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    token_manager: &mut TokenManager,
    voter: &[u8],
) -> StdResult<u128> {
    // filter out not in-progress polls and get max locked
    let mut lock_entries_to_remove: Vec<u64> = vec![];

    let max_locked = token_manager
        .locked_balance
        .iter()
        .filter(|(poll_id, _)| {
            let poll: Poll = poll_read(&deps.storage)
                .load(&poll_id.to_be_bytes())
                .unwrap();

            // cleanup not needed information, voting info in polls with no rewards
            if poll.status != PollStatus::InProgress && poll.voters_reward.is_zero() {
                poll_voter_store(&mut deps.storage, *poll_id).remove(voter);
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

pub fn deposit_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _sender: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let config = config_read(&deps.storage).load()?;

    let mut polls_in_progress = read_polls(
        &deps.storage,
        Some(PollStatus::InProgress),
        None,
        None,
        None,
        Some(true), // remove hard cap to get all polls
    )?;

    if config.voter_weight.is_zero() || polls_in_progress.is_empty() {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![
                log("action", "deposit_reward"),
                log("amount", amount.to_string()),
            ],
            data: None,
        });
    }

    let voter_rewards = amount * config.voter_weight;
    let rewards_per_poll =
        voter_rewards.multiply_ratio(Uint128(1), polls_in_progress.len() as u128);
    if rewards_per_poll.is_zero() {
        return Err(StdError::generic_err("Reward deposited is too small"));
    }
    for poll in polls_in_progress.iter_mut() {
        poll.voters_reward += rewards_per_poll;
        poll_store(&mut deps.storage)
            .save(&poll.id.to_be_bytes(), &poll)
            .unwrap()
    }

    state_store(&mut deps.storage).update(|mut state| {
        state.pending_voting_rewards += voter_rewards;
        Ok(state)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "deposit_reward"),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn withdraw_voting_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = config_store(&mut deps.storage).load()?;
    let sender_address = env.message.sender;
    let key = sender_address.as_str().as_bytes();

    let mut token_manager = bank_read(&deps.storage)
        .load(key)
        .or(Err(StdError::generic_err("Nothing staked")))?;

    let (user_reward_amount, w_polls) =
        withdraw_user_voting_rewards(&mut deps.storage, &sender_address, &token_manager)?;
    if user_reward_amount.eq(&0u128) {
        return Err(StdError::generic_err("Nothing to withdraw"));
    }

    // cleanup, remove from locked_balance the polls from which we withdrew the rewards
    token_manager
        .locked_balance
        .retain(|(poll_id, _)| !w_polls.contains(poll_id));
    bank_store(&mut deps.storage).save(key, &token_manager)?;

    state_store(&mut deps.storage).update(|mut state| {
        state.pending_voting_rewards =
            (state.pending_voting_rewards - Uint128(user_reward_amount))?;
        Ok(state)
    })?;

    send_tokens(
        &config.nebula_token,
        &sender_address,
        user_reward_amount,
        "withdraw_voting_rewards",
    )
}

fn withdraw_user_voting_rewards<S: Storage>(
    storage: &mut S,
    user_address: &HumanAddr,
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

fn get_withdrawable_polls<S: Storage>(
    storage: &S,
    token_manager: &TokenManager,
    user_address: &HumanAddr,
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
    asset_token: &HumanAddr,
    recipient: &HumanAddr,
    amount: u128,
    action: &str,
) -> HandleResult {
    let contract_human = HumanAddr::from(asset_token);
    let recipient_human = HumanAddr::from(recipient);
    let log = vec![
        log("action", action),
        log("recipient", recipient_human.as_str()),
        log("amount", &amount.to_string()),
    ];

    let r = HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_human,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: recipient_human,
                amount: Uint128::from(amount),
            })?,
            send: vec![],
        })],
        log,
        data: None,
    };
    Ok(r)
}

pub fn query_staker<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
) -> StdResult<StakerResponse> {
    let config: Config = config_read(&deps.storage).load()?;
    let state: State = state_read(&deps.storage).load()?;
    let mut token_manager = bank_read(&deps.storage)
        .may_load(address.as_str().as_bytes())?
        .unwrap_or_default();

    // calculate pending voting rewards
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(&deps.storage, &token_manager, &address);
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
        let poll: Poll = poll_read(&deps.storage)
            .load(&poll_id.to_be_bytes())
            .unwrap();

        poll.status == PollStatus::InProgress
    });

    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(&deps, &config.nebula_token, &state.contract_addr)?
        - total_locked_balance)?;

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
        pending_voting_rewards: Uint128(user_reward_amount),
        lock_end_week: token_manager.lock_end_week,
    })
}

// Calculate current voting power of a user
pub fn calc_voting_power(share: Uint128, lock_end_week: u64, current_week: u64) -> Uint128 {
    let locked_weeks_remaining = lock_end_week - current_week;
    let voting_power = (share.u128() * locked_weeks_remaining as u128) / (M as u128);
    return Uint128::from(voting_power);
}

//Increase the number of weeks staked tokens are locked for
pub fn increase_lock_time<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    increase_weeks: u64,
) -> HandleResult {
    let sender_address = env.message.sender;
    let key = sender_address.as_str().as_bytes();

    let mut total_voting_power = total_voting_power_read(&deps.storage).load()?;
    let current_week = env.block.time / SECONDS_PER_WEEK;
    let mut token_manager = bank_read(&deps.storage).may_load(key)?.unwrap_or_default();

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

        bank_store(&mut deps.storage).save(key, &token_manager)?;
        total_voting_power_store(&mut deps.storage).save(&total_voting_power)?;

        Ok(HandleResponse {
            messages: vec![],
            data: None,
            log: vec![
                log("action", "increase_lock_time"),
                log("sender", sender_address.as_str()),
                log("previous_lock_end_week", lock_end_week.to_string()),
                log(
                    "new_lock_end_week",
                    (lock_end_week + increase_weeks).to_string(),
                ),
            ],
        })
    } else {
        Err(StdError::generic_err("User has no tokens staked."))
    }
}

pub fn query_shares<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<SharesResponse> {
    let stakers: Vec<(HumanAddr, TokenManager)> = if let Some(start_after) = start_after {
        read_bank_stakers(&deps.storage, Some(start_after), limit, order_by)?
    } else {
        read_bank_stakers(&deps.storage, None, limit, order_by)?
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
