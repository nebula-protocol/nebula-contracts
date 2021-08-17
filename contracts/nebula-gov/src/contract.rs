use crate::querier::load_token_balance;
use crate::staking::{
    calc_voting_power, deposit_reward, increase_lock_time, query_shares, query_staker,
    stake_voting_tokens, withdraw_voting_rewards, withdraw_voting_tokens, M, SECONDS_PER_WEEK,
};
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_indexer_store, poll_read, poll_store,
    poll_voter_read, poll_voter_store, read_poll_voters, read_polls, state_read, state_store,
    total_voting_power_read, total_voting_power_store, Config, ExecuteData, Poll, State,
    TotalVotingPower,
};
use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use cluster_math::FPDecimal;

use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PollExecuteMsg,
    PollResponse, PollStatus, PollsResponse, QueryMsg, StateResponse, VoteOption, VoterInfo,
    VotersResponse, VotersResponseItem,
};

const MIN_TITLE_LENGTH: usize = 4;
const MAX_TITLE_LENGTH: usize = 64;
const MIN_DESC_LENGTH: usize = 4;
const MAX_DESC_LENGTH: usize = 256;
const MIN_LINK_LENGTH: usize = 12;
const MAX_LINK_LENGTH: usize = 128;
const MAX_POLLS_IN_PROGRESS: usize = 50;

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    validate_quorum(msg.quorum)?;
    validate_threshold(msg.threshold)?;
    validate_voter_weight(msg.voter_weight)?;

    let config = Config {
        nebula_token: msg.nebula_token,
        owner: env.message.sender,
        quorum: msg.quorum,
        threshold: msg.threshold,
        voting_period: msg.voting_period,
        effective_delay: msg.effective_delay,
        expiration_period: msg.expiration_period,
        proposal_deposit: msg.proposal_deposit,
        voter_weight: msg.voter_weight,
        snapshot_period: msg.snapshot_period,
    };

    let state = State {
        contract_addr: env.contract.address,
        poll_count: 0,
        total_share: Uint128::zero(),
        total_deposit: Uint128::zero(),
        pending_voting_rewards: Uint128::zero(),
    };

    let voting_power = TotalVotingPower {
        voting_power: vec![FPDecimal::zero(); M as usize],
        last_upd: (env.block.time.nanos() / 1_000_000_000) / SECONDS_PER_WEEK,
    };

    config_store(deps.storage).save(&config)?;
    state_store(deps.storage).save(&state)?;
    total_voting_power_store(deps.storage).save(&voting_power)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            quorum,
            threshold,
            voting_period,
            effective_delay,
            expiration_period,
            proposal_deposit,
            voter_weight,
            snapshot_period,
        } => update_config(
            deps,
            env,
            owner,
            quorum,
            threshold,
            voting_period,
            effective_delay,
            expiration_period,
            proposal_deposit,
            voter_weight,
            snapshot_period,
        ),
        ExecuteMsg::WithdrawVotingTokens { amount } => withdraw_voting_tokens(deps, env, amount),
        ExecuteMsg::WithdrawVotingRewards {} => withdraw_voting_rewards(deps, env),
        ExecuteMsg::CastVote {
            poll_id,
            vote,
            amount,
        } => cast_vote(deps, env, poll_id, vote, amount),
        ExecuteMsg::EndPoll { poll_id } => end_poll(deps, env, poll_id),
        ExecuteMsg::ExecutePoll { poll_id } => execute_poll(deps, env, poll_id),
        ExecuteMsg::ExpirePoll { poll_id } => expire_poll(deps, env, poll_id),
        ExecuteMsg::SnapshotPoll { poll_id } => snapshot_poll(deps, env, poll_id),
        ExecuteMsg::IncreaseLockTime { increase_weeks } => {
            increase_lock_time(deps, env, increase_weeks)
        }
    }
}

pub fn receive_cw20(deps: DepsMut, env: Env, cw20_msg: Cw20ReceiveMsg) -> StdResult<Response> {
    // only nebula token contract can execute this executer
    let config: Config = config_read(deps.storage).load()?;
    if config.nebula_token != env.message.sender {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(msg) = cw20_msg.msg {
        match from_binary(&msg)? {
            Cw20HookMsg::StakeVotingTokens { lock_for_weeks } => {
                stake_voting_tokens(deps, env, cw20_msg.sender, cw20_msg.amount, lock_for_weeks)
            }
            Cw20HookMsg::CreatePoll {
                title,
                description,
                link,
                execute_msg,
            } => create_poll(
                deps,
                env,
                cw20_msg.sender,
                cw20_msg.amount,
                title,
                description,
                link,
                execute_msg,
            ),
            Cw20HookMsg::DepositReward {} => {
                deposit_reward(deps, env, cw20_msg.sender, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    env: Env,
    owner: Option<String>,
    quorum: Option<Decimal>,
    threshold: Option<Decimal>,
    voting_period: Option<u64>,
    effective_delay: Option<u64>,
    expiration_period: Option<u64>,
    proposal_deposit: Option<Uint128>,
    voter_weight: Option<Decimal>,
    snapshot_period: Option<u64>,
) -> StdResult<Response> {
    config_store(deps.storage).update(|mut config| {
        if config.owner != env.message.sender {
            return Err(StdError::generic_err("unauthorized"));
        }

        if let Some(owner) = owner {
            config.owner = owner;
        }

        if let Some(quorum) = quorum {
            validate_quorum(quorum)?;
            config.quorum = quorum;
        }

        if let Some(threshold) = threshold {
            validate_quorum(threshold)?;
            config.threshold = threshold;
        }

        if let Some(voting_period) = voting_period {
            config.voting_period = voting_period;
        }

        if let Some(effective_delay) = effective_delay {
            config.effective_delay = effective_delay;
        }

        if let Some(expiration_period) = expiration_period {
            config.expiration_period = expiration_period;
        }

        if let Some(proposal_deposit) = proposal_deposit {
            config.proposal_deposit = proposal_deposit;
        }

        if let Some(voter_weight) = voter_weight {
            validate_voter_weight(voter_weight)?;
            config.voter_weight = voter_weight;
        }

        if let Some(snapshot_period) = snapshot_period {
            config.snapshot_period = snapshot_period;
        }

        Ok(config)
    })?;
    Ok(Response::default())
}

/// validate_title returns an error if the title is invalid
fn validate_title(title: &str) -> StdResult<()> {
    if title.len() < MIN_TITLE_LENGTH {
        Err(StdError::generic_err("Title too short"))
    } else if title.len() > MAX_TITLE_LENGTH {
        Err(StdError::generic_err("Title too long"))
    } else {
        Ok(())
    }
}

/// validate_description returns an error if the description is invalid
fn validate_description(description: &str) -> StdResult<()> {
    if description.len() < MIN_DESC_LENGTH {
        Err(StdError::generic_err("Description too short"))
    } else if description.len() > MAX_DESC_LENGTH {
        Err(StdError::generic_err("Description too long"))
    } else {
        Ok(())
    }
}

/// validate_link returns an error if the link is invalid
fn validate_link(link: &Option<String>) -> StdResult<()> {
    if let Some(link) = link {
        if link.len() < MIN_LINK_LENGTH {
            Err(StdError::generic_err("Link too short"))
        } else if link.len() > MAX_LINK_LENGTH {
            Err(StdError::generic_err("Link too long"))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// validate_quorum returns an error if the quorum is invalid
/// (we require 0-1)
fn validate_quorum(quorum: Decimal) -> StdResult<()> {
    if quorum > Decimal::one() {
        Err(StdError::generic_err("quorum must be 0 to 1"))
    } else {
        Ok(())
    }
}

/// validate_threshold returns an error if the threshold is invalid
/// (we require 0-1)
fn validate_threshold(threshold: Decimal) -> StdResult<()> {
    if threshold > Decimal::one() {
        Err(StdError::generic_err("threshold must be 0 to 1"))
    } else {
        Ok(())
    }
}

/// validate_voter_weight returns an error if the voter weight is invalid
/// (we require 0-1)
fn validate_voter_weight(voter_weight: Decimal) -> StdResult<()> {
    if voter_weight > Decimal::one() {
        Err(StdError::generic_err("voter_weight must be smaller than 1"))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
/// create a new poll
pub fn create_poll(
    deps: DepsMut,
    env: Env,
    proposer: String,
    deposit_amount: Uint128,
    title: String,
    description: String,
    link: Option<String>,
    execute_msg: Option<PollExecuteMsg>,
) -> StdResult<Response> {
    validate_title(&title)?;
    validate_description(&description)?;
    validate_link(&link)?;

    let config: Config = config_store(deps.storage).load()?;
    if deposit_amount < config.proposal_deposit {
        return Err(StdError::generic_err(format!(
            "Must deposit more than {} token",
            config.proposal_deposit
        )));
    }

    let polls_in_progress: usize = read_polls(
        deps.storage,
        Some(PollStatus::InProgress),
        None,
        None,
        None,
        Some(true),
    )?
    .len();
    if polls_in_progress.gt(&MAX_POLLS_IN_PROGRESS) {
        return Err(StdError::generic_err("Too many polls in progress"));
    }

    let mut state: State = state_store(deps.storage).load()?;
    let poll_id = state.poll_count + 1;

    // Increase poll count & total deposit amount
    state.poll_count += 1;
    state.total_deposit += deposit_amount;

    let execute_data = if let Some(execute_msg) = execute_msg {
        Some(ExecuteData {
            contract: execute_msg.contract,
            msg: execute_msg.msg,
        })
    } else {
        None
    };

    let new_poll = Poll {
        id: poll_id,
        creator: proposer,
        status: PollStatus::InProgress,
        yes_votes: Uint128::zero(),
        no_votes: Uint128::zero(),
        abstain_votes: Uint128::zero(),
        end_height: env.block.height + config.voting_period,
        title,
        description,
        link,
        execute_data,
        deposit_amount,
        total_balance_at_end_poll: None,
        voters_reward: Uint128::zero(),
        staked_amount: None,
        max_voting_power: Uint128::zero(),
    };

    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &new_poll)?;
    poll_indexer_store(deps.storage, &PollStatus::InProgress)
        .save(&poll_id.to_be_bytes(), &true)?;

    state_store(deps.storage).save(&state)?;

    let r = Response::new().add_attributes(vec![
        attr("action", "create_poll"),
        attr("creator", new_poll.creator),
        attr("poll_id", &poll_id.to_string()),
        attr("end_height", new_poll.end_height),
    ]);
    Ok(r)
}

/*
 * Ends a poll.
 */
pub fn end_poll(deps: DepsMut, env: Env, poll_id: u64) -> StdResult<Response> {
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    if a_poll.status != PollStatus::InProgress {
        return Err(StdError::generic_err("Poll is not in progress"));
    }

    if a_poll.end_height > env.block.height {
        return Err(StdError::generic_err("Voting period has not expired"));
    }

    let no = a_poll.no_votes.u128();
    let yes = a_poll.yes_votes.u128();
    let abstain = a_poll.abstain_votes.u128();

    let tallied_weight = yes + no + abstain;

    let mut poll_status = PollStatus::Rejected;
    let mut rejected_reason = "";
    let mut passed = false;

    let mut messages: Vec<CosmosMsg> = vec![];
    let config: Config = config_read(deps.storage).load()?;
    let mut state: State = state_read(deps.storage).load()?;

    let staked_weight = if let Some(staked_amount) = a_poll.staked_amount {
        staked_amount
    } else {
        a_poll.max_voting_power
    };

    let quorum = if staked_weight != Uint128::zero() {
        Decimal::from_ratio(tallied_weight, staked_weight)
    } else {
        Decimal::zero()
    };

    if tallied_weight == 0 || quorum < config.quorum {
        // Quorum: More than quorum of the total staked tokens at the end of the voting
        // period need to have participated in the vote.
        rejected_reason = "Quorum not reached";
    } else {
        if Decimal::from_ratio(yes, yes + no) > config.threshold {
            //Threshold: More than 50% of the tokens that participated in the vote
            // (after excluding “Abstain” votes) need to have voted in favor of the proposal (“Yes”).
            poll_status = PollStatus::Passed;
            passed = true;
        } else {
            rejected_reason = "Threshold not reached";
        }

        // Refunds deposit only when quorum is reached
        if !a_poll.deposit_amount.is_zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.nebula_token,
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: a_poll.creator.clone(),
                    amount: a_poll.deposit_amount,
                })?,
            }))
        }
    }

    // Decrease total deposit amount
    state.total_deposit = (state.total_deposit - a_poll.deposit_amount)?;
    state_store(deps.storage).save(&state)?;

    // Update poll indexer
    poll_indexer_store(deps.storage, &PollStatus::InProgress).remove(&a_poll.id.to_be_bytes());
    poll_indexer_store(deps.storage, &poll_status).save(&a_poll.id.to_be_bytes(), &true)?;

    // Update poll status
    a_poll.status = poll_status;
    a_poll.total_balance_at_end_poll = Some(staked_weight);
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "end_poll"),
        attr("quorum", quorum),
        attr("tallied_weight", tallied_weight),
        attr("staked_weight", staked_weight),
        attr("poll_id", &poll_id.to_string()),
        attr("rejected_reason", rejected_reason),
        attr("passed", &passed.to_string()),
    ]))
}

/*
 * Execute a msg of passed poll.
 */
pub fn execute_poll(deps: DepsMut, env: Env, poll_id: u64) -> StdResult<Response> {
    let config: Config = config_read(deps.storage).load()?;
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    if a_poll.status != PollStatus::Passed {
        return Err(StdError::generic_err("Poll is not in passed status"));
    }

    if a_poll.end_height + config.effective_delay > env.block.height {
        return Err(StdError::generic_err("Effective delay has not expired"));
    }

    poll_indexer_store(deps.storage, &PollStatus::Passed).remove(&poll_id.to_be_bytes());
    poll_indexer_store(deps.storage, &PollStatus::Executed).save(&poll_id.to_be_bytes(), &true)?;

    a_poll.status = PollStatus::Executed;
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if let Some(execute_data) = a_poll.execute_data {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: execute_data.contract,
            msg: execute_data.msg,
            funds: vec![],
        }))
    } else {
        return Err(StdError::generic_err("The poll does not have execute_data"));
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "execute_poll"),
        attr("poll_id", poll_id.to_string()),
    ]))
}

/// ExpirePoll is used to make the poll as expired state for querying purpose
pub fn expire_poll(deps: DepsMut, env: Env, poll_id: u64) -> StdResult<Response> {
    let config: Config = config_read(deps.storage).load()?;
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    if a_poll.status != PollStatus::Passed {
        return Err(StdError::generic_err("Poll is not in passed status"));
    }

    if a_poll.execute_data.is_none() {
        return Err(StdError::generic_err(
            "Cannot make a text proposal to expired state",
        ));
    }

    if a_poll.end_height + config.expiration_period > env.block.height {
        return Err(StdError::generic_err("Expire height has not been reached"));
    }

    poll_indexer_store(deps.storage, &PollStatus::Passed).remove(&poll_id.to_be_bytes());
    poll_indexer_store(deps.storage, &PollStatus::Expired).save(&poll_id.to_be_bytes(), &true)?;

    a_poll.status = PollStatus::Expired;
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "expire_poll"),
        attr("poll_id", poll_id.to_string()),
    ]))
}

pub fn cast_vote(
    deps: DepsMut,
    env: Env,
    poll_id: u64,
    vote: VoteOption,
    amount: Uint128,
) -> StdResult<Response> {
    let sender_address = env.message.sender;
    let config = config_read(deps.storage).load()?;
    let state = state_read(deps.storage).load()?;
    if poll_id == 0 || state.poll_count < poll_id {
        return Err(StdError::generic_err("Poll does not exist"));
    }

    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Cannot submit zero votes"));
    }

    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;
    if a_poll.status != PollStatus::InProgress || env.block.height > a_poll.end_height {
        return Err(StdError::generic_err("Poll is not in progress"));
    }

    // Check the voter already has a vote on the poll
    if poll_voter_read(deps.storage, poll_id)
        .load(&sender_address.as_str().as_bytes())
        .is_ok()
    {
        return Err(StdError::generic_err("User has already voted."));
    }

    let key = &sender_address.as_str().as_bytes();
    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    // convert share to amount
    let total_share = state.total_share;
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance = (load_token_balance(&deps, &config.nebula_token, &state.contract_addr)?
        - total_locked_balance)?;

    let voting_power = calc_voting_power(
        token_manager
            .share
            .multiply_ratio(total_balance, total_share),
        token_manager.lock_end_week.unwrap(),
        (env.block.time.nanos() / 1_000_000_000) / SECONDS_PER_WEEK,
    );

    if voting_power < amount {
        return Err(StdError::generic_err(
            "User does not have enough staked tokens.",
        ));
    }
    // update tally info
    match vote {
        VoteOption::Yes => a_poll.yes_votes += amount,
        VoteOption::No => a_poll.no_votes += amount,
        VoteOption::Abstain => a_poll.abstain_votes += amount,
    }

    let total_voting_power = total_voting_power_read(deps.storage).load()?;
    // don't need to zero anything out here -- if the user does have voting power then
    // the entry at current_week has to be filled with a valid value
    let current_week = ((env.block.time.nanos() / 1_000_000_000) / SECONDS_PER_WEEK) % M;
    a_poll.max_voting_power = max(
        a_poll.max_voting_power,
        Uint128::from(u128::from(
            total_voting_power.voting_power[current_week as usize],
        )),
    );

    let vote_info = VoterInfo {
        vote,
        balance: amount,
    };
    token_manager
        .locked_balance
        .push((poll_id, vote_info.clone()));
    token_manager.participated_polls = vec![];
    bank_store(deps.storage).save(key, &token_manager)?;

    // store poll voter && and update poll data
    poll_voter_store(deps.storage, poll_id)
        .save(&sender_address.as_str().as_bytes(), &vote_info)?;

    // processing snapshot
    let time_to_end = a_poll.end_height - env.block.height;

    if time_to_end < config.snapshot_period && a_poll.staked_amount.is_none() {
        a_poll.staked_amount = Some(a_poll.max_voting_power);
    }

    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    let log = vec![
        attr("action", "cast_vote"),
        attr("poll_id", &poll_id.to_string()),
        attr("amount", &amount.to_string()),
        attr("voter", sender_address),
        attr("vote_option", vote_info.vote),
    ];

    let r = Response::new().add_attributes(log);
    Ok(r)
}

/// SnapshotPoll is used to take a snapshot of the staked amount for quorum calculation
pub fn snapshot_poll(deps: DepsMut, env: Env, poll_id: u64) -> StdResult<Response> {
    let config: Config = config_read(deps.storage).load()?;
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    if a_poll.status != PollStatus::InProgress {
        return Err(StdError::generic_err("Poll is not in progress"));
    }

    let time_to_end = a_poll.end_height - env.block.height;

    if time_to_end > config.snapshot_period {
        return Err(StdError::generic_err("Cannot snapshot at this height"));
    }

    if a_poll.staked_amount.is_some() {
        return Err(StdError::generic_err("Snapshot has already occurred"));
    }

    let total_voting_power = total_voting_power_read(deps.storage).load()?;
    // don't need to zero anything out here -- if the user does have voting power then
    // the entry at current_week has to be filled with a valid value
    let current_week = ((env.block.time.nanos() / 1_000_000_000) / SECONDS_PER_WEEK) % M;
    a_poll.max_voting_power = max(
        a_poll.max_voting_power,
        Uint128::from(u128::from(
            total_voting_power.voting_power[current_week as usize],
        )),
    );

    a_poll.staked_amount = Some(a_poll.max_voting_power);

    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "snapshot_poll"),
        attr("poll_id", poll_id.to_string()),
        attr("staked_amount", a_poll.max_voting_power),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(&deps)?),
        QueryMsg::State {} => to_binary(&query_state(&deps)?),
        QueryMsg::Staker { address } => to_binary(&query_staker(deps, address)?),
        QueryMsg::Poll { poll_id } => to_binary(&query_poll(deps, poll_id)?),
        QueryMsg::Polls {
            filter,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_polls(deps, filter, start_after, limit, order_by)?),
        QueryMsg::Voters {
            poll_id,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_voters(deps, poll_id, start_after, limit, order_by)?),
        QueryMsg::Shares {
            start_after,
            limit,
            order_by,
        } => to_binary(&query_shares(&deps, start_after, limit, order_by)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = config_read(deps.storage).load()?;
    Ok(ConfigResponse {
        owner: config.owner,
        nebula_token: config.nebula_token,
        quorum: config.quorum,
        threshold: config.threshold,
        voting_period: config.voting_period,
        effective_delay: config.effective_delay,
        expiration_period: config.expiration_period,
        proposal_deposit: config.proposal_deposit,
        voter_weight: config.voter_weight,
        snapshot_period: config.snapshot_period,
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state: State = state_read(deps.storage).load()?;
    Ok(StateResponse {
        poll_count: state.poll_count,
        total_share: state.total_share,
        total_deposit: state.total_deposit,
        pending_voting_rewards: state.pending_voting_rewards,
    })
}

fn query_poll(deps: Deps, poll_id: u64) -> StdResult<PollResponse> {
    let poll = match poll_read(deps.storage).may_load(&poll_id.to_be_bytes())? {
        Some(poll) => Some(poll),
        None => return Err(StdError::generic_err("Poll does not exist")),
    }
    .unwrap();

    Ok(PollResponse {
        id: poll.id,
        creator: poll.creator,
        status: poll.status,
        end_height: poll.end_height,
        title: poll.title,
        description: poll.description,
        link: poll.link,
        deposit_amount: poll.deposit_amount,
        execute_data: if let Some(execute_data) = poll.execute_data {
            Some(PollExecuteMsg {
                contract: execute_data.contract,
                msg: execute_data.msg,
            })
        } else {
            None
        },
        yes_votes: poll.yes_votes,
        no_votes: poll.no_votes,
        abstain_votes: poll.abstain_votes,
        total_balance_at_end_poll: poll.total_balance_at_end_poll,
        voters_reward: poll.voters_reward,
        staked_amount: poll.staked_amount,
    })
}

fn query_polls(
    deps: Deps,
    filter: Option<PollStatus>,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<PollsResponse> {
    let polls = read_polls(deps.storage, filter, start_after, limit, order_by, None)?;
    let poll_responses: StdResult<Vec<PollResponse>> = polls
        .iter()
        .map(|poll| {
            Ok(PollResponse {
                id: poll.id,
                creator: poll.creator.clone(),
                status: poll.status.clone(),
                end_height: poll.end_height,
                title: poll.title.to_string(),
                description: poll.description.to_string(),
                link: poll.link.clone(),
                deposit_amount: poll.deposit_amount,
                execute_data: if let Some(execute_data) = poll.execute_data.clone() {
                    Some(PollExecuteMsg {
                        contract: execute_data.contract,
                        msg: execute_data.msg,
                    })
                } else {
                    None
                },
                yes_votes: poll.yes_votes,
                no_votes: poll.no_votes,
                abstain_votes: poll.abstain_votes,
                total_balance_at_end_poll: poll.total_balance_at_end_poll,
                voters_reward: poll.voters_reward,
                staked_amount: poll.staked_amount,
            })
        })
        .collect();

    Ok(PollsResponse {
        polls: poll_responses?,
    })
}

fn query_voters(
    deps: Deps,
    poll_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<VotersResponse> {
    let poll: Poll = match poll_read(deps.storage).may_load(&poll_id.to_be_bytes())? {
        Some(poll) => Some(poll),
        None => return Err(StdError::generic_err("Poll does not exist")),
    }
    .unwrap();

    let voters = if poll.status != PollStatus::InProgress {
        vec![]
    } else if let Some(start_after) = start_after {
        read_poll_voters(deps.storage, poll_id, Some(start_after), limit, order_by)?
    } else {
        read_poll_voters(deps.storage, poll_id, None, limit, order_by)?
    };

    let voters_response: StdResult<Vec<VotersResponseItem>> = voters
        .iter()
        .map(|voter_info| {
            Ok(VotersResponseItem {
                voter: voter_info.0.clone(),
                vote: voter_info.1.vote.clone(),
                balance: voter_info.1.balance,
            })
        })
        .collect();

    Ok(VotersResponse {
        voters: voters_response?,
    })
}

use std::cmp::max;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
