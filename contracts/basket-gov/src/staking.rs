use crate::querier::load_token_balance;
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_read, poll_store, poll_voter_read,
    poll_voter_store, read_polls, state_read, state_store, Config, Poll, State, TokenManager,
};
use std::convert::{TryFrom};
use std::cmp;

use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
use nebula_protocol::gov::{PollStatus, StakerResponse, VoterInfo};

static SECONDS_PER_WEEK: u128 = 604800u128; //60 * 60 * 24 * 7
static M : u128 = 104; //Max weeks

pub fn stake_voting_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    amount: Uint128,
    lock_for_weeks: u128
) -> HandleResult {
    if amount.is_zero() {
        return Err(StdError::generic_err("Insufficient funds sent"));
    }

    let sender_address_raw = deps.api.canonical_address(&sender)?;
    let key = &sender_address_raw.as_slice();

    let mut token_manager = bank_read(&deps.storage).may_load(key)?.unwrap_or_default();
    let config: Config = config_store(&mut deps.storage).load()?;
    let mut state: State = state_store(&mut deps.storage).load()?;

    // balance already increased, so subtract deposit amount
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(
        &deps,
        &deps.api.human_address(&config.nebula_token)?,
        &state.contract_addr,
    )? - (total_locked_balance + amount))?;

    let share = if total_balance.is_zero() || state.total_share.is_zero() {
        amount
    } else {
        amount.multiply_ratio(state.total_share, total_balance)
    };
    ////////// Time-weighted staking start ////////
    if let Some(mut lock_end_time) = token_manager.lock_end_time {
        let locked_seconds_remaining = Uint128::from(
            cmp::max(lock_end_time, env.block.time) - env.block.time
        ).u128(); //W
        let lock_for_seconds = Uint128::from(lock_for_weeks * SECONDS_PER_WEEK).u128(); //W_n
        let new_share = token_manager.share + share;
        let new_locked_seconds = (
            locked_seconds_remaining * token_manager.share.u128() + lock_for_seconds * share.u128()
        )/new_share.u128();

        //Round new_locked_seconds to nearest week
        let rounded_locked_seconds = (
            (new_locked_seconds + SECONDS_PER_WEEK - 1)/SECONDS_PER_WEEK
        ) * SECONDS_PER_WEEK;
        lock_end_time = u64::try_from(rounded_locked_seconds).unwrap() + env.block.time;
        token_manager.lock_end_time = Some(lock_end_time);
    } else {
        let lock_end_time = u64::try_from(lock_for_weeks * SECONDS_PER_WEEK).unwrap() + env.block.time;
        token_manager.lock_end_time = Some(lock_end_time);
    }
    ////////// Time-weighted staking end ////////
    
    token_manager.share += share;
    state.total_share += share;

    state_store(&mut deps.storage).save(&state)?;
    bank_store(&mut deps.storage).save(key, &token_manager)?;

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
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let key = sender_address_raw.as_slice();

    if let Some(mut token_manager) = bank_read(&deps.storage).may_load(key)? {
        let config: Config = config_store(&mut deps.storage).load()?;
        let mut state: State = state_store(&mut deps.storage).load()?;

        // Load total share & total balance except proposal deposit amount
        let total_share = state.total_share.u128();
        let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
        let total_balance = (load_token_balance(
            &deps,
            &deps.api.human_address(&config.nebula_token)?,
            &state.contract_addr,
        )? - total_locked_balance)?
            .u128();

        let user_locked_balance = compute_locked_balance(deps, &mut token_manager)?;
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
        } else if env.block.time < token_manager.lock_end_time.unwrap() {
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
                &deps.api,
                &config.nebula_token,
                &sender_address_raw,
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
) -> StdResult<u128> {
    // filter out not in-progress polls
    token_manager.locked_balance.retain(|(poll_id, _)| {
        let poll: Poll = poll_read(&deps.storage)
            .load(&poll_id.to_be_bytes())
            .unwrap();

        poll.status == PollStatus::InProgress
    });

    Ok(token_manager
        .locked_balance
        .iter()
        .map(|(_, v)| v.balance.u128())
        .max()
        .unwrap_or_default())
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
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let key = sender_address_raw.as_slice();

    let token_manager = bank_read(&deps.storage)
        .load(key)
        .or(Err(StdError::generic_err("Nothing staked")))?;

    let user_reward_amount: u128 =
        withdraw_user_voting_rewards(&mut deps.storage, &sender_address_raw, &token_manager);
    if user_reward_amount.eq(&0u128) {
        return Err(StdError::generic_err("Nothing to withdraw"));
    }

    state_store(&mut deps.storage).update(|mut state| {
        state.pending_voting_rewards =
            (state.pending_voting_rewards - Uint128(user_reward_amount))?;
        Ok(state)
    })?;

    send_tokens(
        &deps.api,
        &config.nebula_token,
        &sender_address_raw,
        user_reward_amount,
        "withdraw_voting_rewards",
    )
}

pub fn stake_voting_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = config_store(&mut deps.storage).load()?;
    let mut state: State = state_store(&mut deps.storage).load()?;
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let key = sender_address_raw.as_slice();

    let mut token_manager = bank_read(&deps.storage)
        .load(key)
        .or(Err(StdError::generic_err("Nothing staked")))?;

    let user_reward_amount: u128 =
        withdraw_user_voting_rewards(&mut deps.storage, &sender_address_raw, &token_manager);
    if user_reward_amount.eq(&0u128) {
        return Err(StdError::generic_err("Nothing to withdraw"));
    }

    // add the withdrawn rewards to stake pool and calculate share
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(
        &deps,
        &deps.api.human_address(&config.nebula_token)?,
        &state.contract_addr,
    )? - total_locked_balance)?;

    state.pending_voting_rewards = (state.pending_voting_rewards - Uint128(user_reward_amount))?;

    let share: Uint128 = if total_balance.is_zero() || state.total_share.is_zero() {
        Uint128(user_reward_amount)
    } else {
        Uint128(user_reward_amount).multiply_ratio(state.total_share, total_balance)
    };

    token_manager.share += share;
    state.total_share += share;

    state_store(&mut deps.storage).save(&state)?;
    bank_store(&mut deps.storage).save(key, &token_manager)?;

    Ok(HandleResponse {
        messages: vec![],
        data: None,
        log: vec![
            log("action", "stake_voting_rewards"),
            log("staker", env.message.sender.as_str()),
            log("share", share.to_string()),
            log("amount", user_reward_amount.to_string()),
        ],
    })
}

fn withdraw_user_voting_rewards<S: Storage>(
    storage: &mut S,
    user_address: &CanonicalAddr,
    token_manager: &TokenManager,
) -> u128 {
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(storage, token_manager, user_address);
    let user_reward_amount: u128 = w_polls
        .iter()
        .map(|(poll, voting_info)| {
            // remove voter info from the poll
            poll_voter_store(storage, poll.id).remove(user_address.as_slice());

            // calculate reward share
            let total_votes =
                poll.no_votes.u128() + poll.yes_votes.u128() + poll.abstain_votes.u128();
            let poll_voting_reward = poll
                .voters_reward
                .multiply_ratio(voting_info.balance, total_votes);
            poll_voting_reward.u128()
        })
        .sum();
    user_reward_amount
}

fn get_withdrawable_polls<S: Storage>(
    storage: &S,
    token_manager: &TokenManager,
    user_address: &CanonicalAddr,
) -> Vec<(Poll, VoterInfo)> {
    let w_polls: Vec<(Poll, VoterInfo)> = token_manager
        .locked_balance
        .iter()
        .map(|(poll_id, _)| {
            let poll: Poll = poll_read(storage).load(&poll_id.to_be_bytes()).unwrap();
            let voter_info_res: StdResult<VoterInfo> =
                poll_voter_read(storage, *poll_id).load(&user_address.as_slice());
            (poll, voter_info_res)
        })
        .filter(|(poll, voter_info_res)| {
            poll.status != PollStatus::InProgress && voter_info_res.is_ok()
        })
        .map(|(poll, voter_info_res)| (poll, voter_info_res.unwrap()))
        .collect();
    w_polls
}

fn send_tokens<A: Api>(
    api: &A,
    asset_token: &CanonicalAddr,
    recipient: &CanonicalAddr,
    amount: u128,
    action: &str,
) -> HandleResult {
    let contract_human = api.human_address(asset_token)?;
    let recipient_human = api.human_address(recipient)?;
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
    let addr_raw = deps.api.canonical_address(&address).unwrap();
    let config: Config = config_read(&deps.storage).load()?;
    let state: State = state_read(&deps.storage).load()?;
    let mut token_manager = bank_read(&deps.storage)
        .may_load(addr_raw.as_slice())?
        .unwrap_or_default();

    // calculate pending voting rewards
    let w_polls: Vec<(Poll, VoterInfo)> =
        get_withdrawable_polls(&deps.storage, &token_manager, &addr_raw);
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
    let total_balance = (load_token_balance(
        &deps,
        &deps.api.human_address(&config.nebula_token)?,
        &state.contract_addr,
    )? - total_locked_balance)?;

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
        lock_end_time: token_manager.lock_end_time
    })
}

// Calculate current voting power of a user
pub fn calc_voting_power(share: Uint128, lock_end_time: u64, time: u64) -> Uint128 {
    let locked_seconds_remaining = Uint128::from(lock_end_time - time).u128();
    let locked_weeks_remaining = (locked_seconds_remaining + SECONDS_PER_WEEK - 1)/SECONDS_PER_WEEK;
    let voting_power = (share.u128() * locked_weeks_remaining) / M;
    return Uint128::from(voting_power);
}