#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::querier::load_token_balance;
use crate::staking::{
    deposit_reward, query_shares, query_staker, stake_voting_rewards, stake_voting_tokens,
    withdraw_voting_rewards, withdraw_voting_tokens,
};
use crate::state::{
    bank_read, bank_store, config_read, config_store, poll_indexer_store, poll_read, poll_store,
    poll_voter_read, poll_voter_store, read_poll_voters, read_polls, read_tmp_poll_id, state_read,
    state_store, store_tmp_poll_id, Config, ExecuteData, Poll, State,
};

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PollExecuteMsg,
    PollResponse, PollStatus, PollsResponse, QueryMsg, StateResponse, VoteOption, VoterInfo,
    VotersResponse, VotersResponseItem,
};

/// Poll's title minimum length
const MIN_TITLE_LENGTH: usize = 4;
/// Poll's title maximum length
const MAX_TITLE_LENGTH: usize = 64;
/// Poll's description minimum length
const MIN_DESC_LENGTH: usize = 4;
/// Poll's description maximum length
const MAX_DESC_LENGTH: usize = 256;
/// Poll's link minimum length
const MIN_LINK_LENGTH: usize = 12;
/// Poll's link maximum length
const MAX_LINK_LENGTH: usize = 128;
/// Maximum number of open polls at a time
const MAX_POLLS_IN_PROGRESS: usize = 50;

/// Reply ID of a submessage in execute poll
const POLL_EXECUTE_REPLY_ID: u64 = 1;

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validate values to be between 0-1
    validate_quorum(msg.quorum)?;
    validate_threshold(msg.threshold)?;
    validate_voter_weight(msg.voter_weight)?;

    // Populate the contract setting from the message
    let config = Config {
        nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
        owner: info.sender,
        quorum: msg.quorum,
        threshold: msg.threshold,
        voting_period: msg.voting_period,
        effective_delay: msg.effective_delay,
        expiration_period: 0u64, // deprecated
        proposal_deposit: msg.proposal_deposit,
        voter_weight: msg.voter_weight,
        snapshot_period: msg.snapshot_period,
    };

    // Initialize the contract state
    let state = State {
        contract_addr: env.contract.address,
        poll_count: 0,
        total_share: Uint128::zero(),
        total_deposit: Uint128::zero(),
        pending_voting_rewards: Uint128::zero(),
    };

    config_store(deps.storage).save(&config)?;
    state_store(deps.storage).save(&state)?;

    Ok(Response::default())
}

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::Receive (msg)** Receives CW20 tokens and executes a hook message.
///
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             quorum,
///             threshold,
///             voting_period,
///             effective_delay,
///             proposal_deposit,
///             voter_weight,
///             snapshot_period,
///         }** Updates general governance contract parameters.
///
/// - **ExecuteMsg::WithdrawVotingTokens {
///             amount,
///         }** Withdraws `amount` of Nebula token if not staked.
///
/// - **ExecuteMsg::WithdrawVotingRewards {
///             poll_id,
///         }** Withdraws rewards from `poll_id` or all rewards if not specified.
///
/// - **ExecuteMsg::StakeVotingRewards {
///             poll_id,
///         }** Stakes rewards from `poll_id` or all rewards if not specified.
///
/// - **ExecuteMsg::CastVote {
///             poll_id,
///             vote,
///             amount,
///         }** Casts vote on a poll with the specified `amount`.
///
/// - **ExecuteMsg::EndPoll {
///             poll_id,
///         }** Ends an on-going poll.
///
/// - **ExecuteMsg::ExecutePoll {
///             poll_id,
///         }** Executes a poll.
///
/// - **ExecuteMsg::SnapshotPoll {
///             poll_id,
///         }** Snapshots a poll.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            quorum,
            threshold,
            voting_period,
            effective_delay,
            proposal_deposit,
            voter_weight,
            snapshot_period,
        } => update_config(
            deps,
            info,
            owner,
            quorum,
            threshold,
            voting_period,
            effective_delay,
            proposal_deposit,
            voter_weight,
            snapshot_period,
        ),
        ExecuteMsg::WithdrawVotingTokens { amount } => withdraw_voting_tokens(deps, info, amount),
        ExecuteMsg::WithdrawVotingRewards { poll_id } => {
            withdraw_voting_rewards(deps, info, poll_id)
        }
        ExecuteMsg::StakeVotingRewards { poll_id } => stake_voting_rewards(deps, info, poll_id),
        ExecuteMsg::CastVote {
            poll_id,
            vote,
            amount,
        } => cast_vote(deps, env, info, poll_id, vote, amount),
        ExecuteMsg::EndPoll { poll_id } => end_poll(deps, env, poll_id),
        ExecuteMsg::ExecutePoll { poll_id } => execute_poll(deps, env, poll_id),
        ExecuteMsg::SnapshotPoll { poll_id } => snapshot_poll(deps, env, poll_id),
    }
}

/// ## Description
/// Receives CW20 tokens and executes a hook message.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cw20_msg** is an object of type [`Cw20ReceiveMsg`] which is a hook message to be executed.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config: Config = config_read(deps.storage).load()?;

    // Permission check, only Nebula token contract can execute this message
    if config.nebula_token != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    match from_binary(&cw20_msg.msg) {
        // If `StakeVotingTokens`, sender stakes Nebula tokens for the specified amount
        Ok(Cw20HookMsg::StakeVotingTokens {}) => {
            stake_voting_tokens(deps, cw20_msg.sender, cw20_msg.amount)
        }
        // If `CreatePoll`, sender creates a poll as the poll proposer
        // and the receive amount as the poll deposit amount
        Ok(Cw20HookMsg::CreatePoll {
            title,
            description,
            link,
            execute_msgs,
        }) => create_poll(
            deps,
            env,
            cw20_msg.sender,
            cw20_msg.amount,
            title,
            description,
            link,
            execute_msgs,
        ),
        // If `DepositReward`, deposits a reward to stakers and in-progress polls
        Ok(Cw20HookMsg::DepositReward {}) => deposit_reward(deps, cw20_msg.amount),
        Err(_) => Err(ContractError::Generic(
            "invalid cw20 hook message".to_string(),
        )),
    }
}

/// ## Description
/// Exposes all the reply callback function available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`Reply`] which is a response message returned
///     from executing a submessage.
///
/// ## Message IDs
/// - **POLL_EXECUTE_REPLY_ID (1)** Receives only if execute poll fails and marks the poll as failed.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        POLL_EXECUTE_REPLY_ID => {
            // Get the executed poll ID
            let poll_id: u64 = read_tmp_poll_id(deps.storage)?;
            // Mark the poll as failed
            failed_poll(deps, poll_id)
        }
        _ => Err(ContractError::Generic("reply id is invalid".to_string())),
    }
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is an owner to update.
///
/// - **quorum** is an object of type [`Option<String>`] which is a poll quorum to update.
///
/// - **threshold** is an object of type [`Option<String>`] which is a pass threshold for a poll.
///
/// - **voting_period** is an object of type [`Option<u64>`] which is a poll voting period.
///
/// - **effective_day** is an object of type [`Option<u64>`] which is a new delay time for
///     a poll to be executed after reaching the voting period.
///
/// - **proposal_deposit** is an object of type [`Option<Uint128>`] which is a minimum deposit
///     when creating a poll.
///
/// - **voter_weight** is an object of type [`Option<Decimal>`] which is a ratio of a total reward
///     distributed to voters.
///
/// - **snapshot_period** is an object of type [`Option<u64>`] which is a snapshot time to lock
///     the current quorum of the poll.
///
/// ## Executor
/// Only the owner can execute this.
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    quorum: Option<Decimal>,
    threshold: Option<Decimal>,
    voting_period: Option<u64>,
    effective_delay: Option<u64>,
    proposal_deposit: Option<Uint128>,
    voter_weight: Option<Decimal>,
    snapshot_period: Option<u64>,
) -> Result<Response, ContractError> {
    let api = deps.api;
    config_store(deps.storage).update(|mut config| {
        // Permission check
        if config.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        if let Some(owner) = owner {
            // Validate contract address
            config.owner = api.addr_validate(owner.as_str())?;
        }

        if let Some(quorum) = quorum {
            // Validate value to be between 0-1
            validate_quorum(quorum)?;
            config.quorum = quorum;
        }

        if let Some(threshold) = threshold {
            // Validate value to be between 0-1
            validate_threshold(threshold)?;
            config.threshold = threshold;
        }

        if let Some(voting_period) = voting_period {
            config.voting_period = voting_period;
        }

        if let Some(effective_delay) = effective_delay {
            config.effective_delay = effective_delay;
        }

        if let Some(proposal_deposit) = proposal_deposit {
            config.proposal_deposit = proposal_deposit;
        }

        if let Some(voter_weight) = voter_weight {
            // Validate value to be between 0-1
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

/// ## Description
/// Returns an error if the title is invalid.
///
/// ## Params
/// - **title** is a reference to an object of type [`str`].
fn validate_title(title: &str) -> Result<(), ContractError> {
    if title.len() < MIN_TITLE_LENGTH {
        Err(ContractError::ValueTooShort("Title".to_string()))
    } else if title.len() > MAX_TITLE_LENGTH {
        Err(ContractError::ValueTooLong("Title".to_string()))
    } else {
        Ok(())
    }
}

/// ## Description
/// Returns an error if the description is invalid.
///
/// ## Params
/// - **description** is a reference to an object of type [`str`].
fn validate_description(description: &str) -> Result<(), ContractError> {
    if description.len() < MIN_DESC_LENGTH {
        Err(ContractError::ValueTooShort("Description".to_string()))
    } else if description.len() > MAX_DESC_LENGTH {
        Err(ContractError::ValueTooLong("Description".to_string()))
    } else {
        Ok(())
    }
}

/// ## Description
/// Returns an error if link is not none but invalid.
///
/// ## Params
/// - **link** is a reference to an object of type [`Option<String>`].
fn validate_link(link: &Option<String>) -> Result<(), ContractError> {
    if let Some(link) = link {
        if link.len() < MIN_LINK_LENGTH {
            Err(ContractError::ValueTooShort("Link".to_string()))
        } else if link.len() > MAX_LINK_LENGTH {
            Err(ContractError::ValueTooLong("Link".to_string()))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// ## Description
/// Returns an error if the quorum is invalid, require 0-1.
///
/// ## Params
/// - **quorum** is a reference to an object of type [`Decimal`].
fn validate_quorum(quorum: Decimal) -> Result<(), ContractError> {
    if quorum > Decimal::one() {
        Err(ContractError::ValueOutOfRange(
            "quorum".to_string(),
            Uint128::new(0),
            Uint128::new(1),
        ))
    } else {
        Ok(())
    }
}

/// ## Description
/// Returns an error if the threshold is invalid, require 0-1.
///
/// ## Params
/// - **threshold** is a reference to an object of type [`Decimal`].
fn validate_threshold(threshold: Decimal) -> Result<(), ContractError> {
    if threshold > Decimal::one() {
        Err(ContractError::ValueOutOfRange(
            "threshold".to_string(),
            Uint128::new(0),
            Uint128::new(1),
        ))
    } else {
        Ok(())
    }
}

/// ## Description
/// Returns an error if the voter weight is invalid, require 0-1.
///
/// ## Params
/// - **voter_weight** is a reference to an object of type [`Decimal`].
pub fn validate_voter_weight(voter_weight: Decimal) -> Result<(), ContractError> {
    if voter_weight >= Decimal::one() {
        Err(ContractError::Generic(
            "voter_weight must be smaller than 1".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// ## Description
/// Creates a new poll.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **proposer** is an object of type [`String`] which is the address of the poll proposer.
///
/// - **deposit_amount** is an object of type [`Uint128`] which is the poll deposit amount.
///
/// - **title** is an object of type [`String`] which is the poll title.
///
/// - **description** is an object of type [`String`] which is the poll description.
///
/// - **link** is an object of type [`Option<String>`] which is the poll related information link.
///
/// - **poll_execute_msg** is an object of type [`Option<PollExecuteMsg>`] which is the message
///     to be executed if the poll succeeds.
#[allow(clippy::too_many_arguments)]
pub fn create_poll(
    deps: DepsMut,
    env: Env,
    proposer: String,
    deposit_amount: Uint128,
    title: String,
    description: String,
    link: Option<String>,
    poll_execute_msgs: Option<Vec<PollExecuteMsg>>,
) -> Result<Response, ContractError> {
    // Validate srting values
    validate_title(&title)?;
    validate_description(&description)?;
    validate_link(&link)?;

    let config: Config = config_store(deps.storage).load()?;

    // Check if deposit amount is more than the deposit threshold
    if deposit_amount < config.proposal_deposit {
        return Err(ContractError::Generic(format!(
            "Must deposit more than {} token",
            config.proposal_deposit
        )));
    }

    // Check if there are not too many polls in progress
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
        return Err(ContractError::Generic(
            "Too many polls in progress".to_string(),
        ));
    }

    let mut state: State = state_store(deps.storage).load()?;
    let poll_id = state.poll_count + 1;

    // Increase poll count & total deposit amount
    state.poll_count += 1;
    state.total_deposit += deposit_amount;

    // Extract the execute data from the message if any
    let poll_execute_data = if let Some(poll_execute_msgs) = poll_execute_msgs {
        Some(
            poll_execute_msgs
                .iter()
                .map(|poll_execute_msg| -> ExecuteData {
                    ExecuteData {
                        contract: deps
                            .api
                            .addr_validate(poll_execute_msg.contract.as_str())
                            .unwrap(),
                        msg: poll_execute_msg.msg.clone(),
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Validate address format
    let sender_address = deps.api.addr_validate(proposer.as_str())?;
    let current_seconds = env.block.time.seconds();

    // Create the poll
    let new_poll = Poll {
        id: poll_id,
        creator: sender_address.clone(),
        status: PollStatus::InProgress,
        yes_votes: Uint128::zero(),
        no_votes: Uint128::zero(),
        abstain_votes: Uint128::zero(),
        end_time: current_seconds + config.voting_period,
        title,
        description,
        link,
        execute_data: poll_execute_data,
        deposit_amount,
        total_balance_at_end_poll: None,
        voters_reward: Uint128::zero(),
        staked_amount: None,
    };

    // Save the poll
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &new_poll)?;
    // Set the poll status to be in-progress
    poll_indexer_store(deps.storage, &PollStatus::InProgress)
        .save(&poll_id.to_be_bytes(), &true)?;

    // Update the governance all poll state
    state_store(deps.storage).save(&state)?;

    let r = Response::new().add_attributes(vec![
        attr("action", "create_poll"),
        attr("creator", sender_address.to_string()),
        attr("poll_id", &poll_id.to_string()),
        attr("end_time", new_poll.end_time.to_string()),
    ]);
    Ok(r)
}

/// ## Description
/// Ends a poll.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **poll_id** is an object of type [`u64`].
pub fn end_poll(deps: DepsMut, env: Env, poll_id: u64) -> Result<Response, ContractError> {
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    // Can only end an in-progress poll
    if a_poll.status != PollStatus::InProgress {
        return Err(ContractError::PollNotInProgress {});
    }

    let current_seconds = env.block.time.seconds();

    // Can only end a poll past its voting period
    if a_poll.end_time > current_seconds {
        return Err(ContractError::ValueHasNotExpired(
            "Voting period".to_string(),
        ));
    }

    // Calculate vote
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

    let (quorum, staked_weight) = if state.total_share.u128() == 0 {
        // If there is no staked, `quorum` and `staked_weight` are 0
        (Decimal::zero(), Uint128::zero())
    } else if let Some(staked_amount) = a_poll.staked_amount {
        // If a snapshot is made, find `quorom` and `stake_weight` from the total stake at snapshot
        (
            Decimal::from_ratio(tallied_weight, staked_amount),
            staked_amount,
        )
    } else {
        // If no snapshot is not made, calculate the current total stake
        // Governance total Nebula balance = total stake + total deposit + all voting rewards
        let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
        let staked_weight =
            load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
                .checked_sub(total_locked_balance)?;
        // Compute `quorum` and `staked_weight`
        (
            Decimal::from_ratio(tallied_weight, staked_weight),
            staked_weight,
        )
    };

    if tallied_weight == 0 || quorum < config.quorum {
        // Quorum: More than quorum of the total staked tokens at the end of the voting
        // period need to have participated in the vote.
        rejected_reason = "Quorum not reached";
    } else {
        if yes != 0u128 && Decimal::from_ratio(yes, yes + no) > config.threshold {
            // Threshold: More than 50% of the tokens that participated in the vote
            // (after excluding “Abstain” votes) need to have voted in favor of the proposal (“Yes”).
            poll_status = PollStatus::Passed;
            passed = true;
        } else {
            rejected_reason = "Threshold not reached";
        }

        // Refund deposit only when quorum is reached
        if !a_poll.deposit_amount.is_zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.nebula_token.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: a_poll.creator.to_string(),
                    amount: a_poll.deposit_amount,
                })?,
            }))
        }
    }

    // Decrease total deposit amount
    state.total_deposit = state.total_deposit.checked_sub(a_poll.deposit_amount)?;
    state_store(deps.storage).save(&state)?;

    // Update poll indexer
    // - Remove the poll from the in-progress index
    poll_indexer_store(deps.storage, &PollStatus::InProgress).remove(&a_poll.id.to_be_bytes());
    // - Add the poll to the final poll status index
    poll_indexer_store(deps.storage, &poll_status).save(&a_poll.id.to_be_bytes(), &true)?;

    // Update poll status
    a_poll.status = poll_status;
    a_poll.total_balance_at_end_poll = Some(staked_weight);
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "end_poll"),
        attr("poll_id", &poll_id.to_string()),
        attr("rejected_reason", rejected_reason),
        attr("passed", &passed.to_string()),
    ]))
}

/// ## Description
/// Executes a msg of passed poll.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **poll_id** is an object of type [`u64`].
pub fn execute_poll(deps: DepsMut, env: Env, poll_id: u64) -> Result<Response, ContractError> {
    let config: Config = config_read(deps.storage).load()?;
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    // Can only execute a passed poll
    if a_poll.status != PollStatus::Passed {
        return Err(ContractError::Generic(
            "Poll is not in passed status".to_string(),
        ));
    }

    // Need to wait after the effective delay before executing the poll
    let current_seconds = env.block.time.seconds();
    if a_poll.end_time + config.effective_delay > current_seconds {
        return Err(ContractError::ValueHasNotExpired(
            "Effective delay".to_string(),
        ));
    }

    // Check if there is execute data in the poll
    if a_poll.execute_data.is_none() {
        return Err(ContractError::Generic(
            "Poll has no execute data".to_string(),
        ));
    }

    // Update poll indexer of the poll from Passed to Executed
    poll_indexer_store(deps.storage, &PollStatus::Passed).remove(&poll_id.to_be_bytes());
    poll_indexer_store(deps.storage, &PollStatus::Executed).save(&poll_id.to_be_bytes(), &true)?;

    // Update poll status
    a_poll.status = PollStatus::Executed;
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    // Retrieve the execute messages
    let messages: Vec<SubMsg> = a_poll.execute_data.unwrap().iter().map(fill_msg).collect();
    store_tmp_poll_id(deps.storage, a_poll.id)?;

    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(vec![
            attr("action", "execute_poll"),
            attr("poll_id", poll_id.to_string()),
        ]))
}

/// ## Description
/// Extracts `ExecuteData` into Cosmwasm `SubMsg`.
///
/// ## Params
/// - **msg** is a reference to an object of type [`ExecuteData`].
fn fill_msg(msg: &ExecuteData) -> SubMsg {
    match msg {
        ExecuteData { contract, msg } => SubMsg {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.to_string(),
                msg: msg.clone(),
                funds: vec![],
            }),
            gas_limit: None,
            id: POLL_EXECUTE_REPLY_ID,
            reply_on: ReplyOn::Error,
        },
    }
}

/// ## Description
/// If the executed message of a passed poll fails, it is marked as failed
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **poll_id** is an object of type [`u64`].
pub fn failed_poll(deps: DepsMut, poll_id: u64) -> Result<Response, ContractError> {
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;

    // Update poll indexer of the poll from Executed to Failed
    poll_indexer_store(deps.storage, &PollStatus::Executed).remove(&poll_id.to_be_bytes());
    poll_indexer_store(deps.storage, &PollStatus::Failed).save(&poll_id.to_be_bytes(), &true)?;

    a_poll.status = PollStatus::Failed;
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_attribute("action", "failed_poll"))
}

/// ## Description
/// User casts a vote on the provided poll ID.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **poll_id** is an object of type [`u64`].
///
/// - **vote** is an object of type [`VoteOption`].
///
/// - **amount** is an object of type [`Uint128`].
pub fn cast_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    poll_id: u64,
    vote: VoteOption,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender_address = info.sender;
    let config = config_read(deps.storage).load()?;
    let state = state_read(deps.storage).load()?;

    // Validate the poll ID
    if poll_id == 0 || state.poll_count < poll_id {
        return Err(ContractError::PollNotExists {});
    }

    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;
    let current_seconds = env.block.time.seconds();
    // Can only cast vote on an in-progress poll that is not past the voting period
    if a_poll.status != PollStatus::InProgress || current_seconds > a_poll.end_time {
        return Err(ContractError::PollNotInProgress {});
    }
    let key = sender_address.as_bytes();

    // Check the voter already has a vote on the poll
    if poll_voter_read(deps.storage, poll_id).load(&key).is_ok() {
        return Err(ContractError::Generic("User has already voted".to_string()));
    }

    // Load voter token manager
    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    // Convert the voter share to the voter actual staked amount
    let total_share = state.total_share;
    // Governance total Nebula balance = total stake + total deposit + all voting rewards
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let total_balance =
        load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance)?;

    // Compute voter staked = voter_share * total_stake / total_share
    if token_manager
        .share
        .multiply_ratio(total_balance, total_share)
        < amount
    {
        return Err(ContractError::Generic(
            "User does not have enough staked tokens".to_string(),
        ));
    }

    // Update tally info
    match vote {
        VoteOption::Yes => a_poll.yes_votes += amount,
        VoteOption::No => a_poll.no_votes += amount,
        VoteOption::Abstain => a_poll.abstain_votes += amount,
    }

    let vote_info = VoterInfo {
        vote,
        balance: amount,
    };
    token_manager
        .locked_balance
        .push((poll_id, vote_info.clone()));
    token_manager.participated_polls = vec![];
    bank_store(deps.storage).save(key, &token_manager)?;

    // Store poll voter
    poll_voter_store(deps.storage, poll_id).save(key, &vote_info)?;

    // Processing snapshot
    let time_to_end = a_poll.end_time - current_seconds;
    if time_to_end < config.snapshot_period && a_poll.staked_amount.is_none() {
        a_poll.staked_amount = Some(total_balance);
    }

    // Update poll data
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "cast_vote"),
        attr("poll_id", &poll_id.to_string()),
        attr("amount", &amount.to_string()),
        attr("voter", sender_address.to_string()),
        attr("vote_option", vote_info.vote.to_string()),
    ]))
}

/// ## Description
/// Takes a snapshot of the current staked amount for quorum calculation.
/// - Can only be executed once per poll.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **poll_id** is an object of tyoe [`u64`].
pub fn snapshot_poll(deps: DepsMut, env: Env, poll_id: u64) -> Result<Response, ContractError> {
    let config: Config = config_read(deps.storage).load()?;
    // Get the poll and check if the status is in-progress
    let mut a_poll: Poll = poll_store(deps.storage).load(&poll_id.to_be_bytes())?;
    if a_poll.status != PollStatus::InProgress {
        return Err(ContractError::PollNotInProgress {});
    }

    // Check if we have reached the snapshot period
    let current_seconds = env.block.time.seconds();
    let time_to_end = a_poll.end_time - current_seconds;

    if time_to_end > config.snapshot_period {
        return Err(ContractError::Generic(
            "Cannot snapshot at this height".to_string(),
        ));
    }

    // Check that there is no snapshot made yet
    if a_poll.staked_amount.is_some() {
        return Err(ContractError::Generic(
            "Snapshot has already occurred".to_string(),
        ));
    }

    // Store the current staked amount for quorum calculation at end poll
    let state: State = state_store(deps.storage).load()?;

    // Governance total Nebula balance = total stake + total deposit + all voting rewards
    let total_locked_balance = state.total_deposit + state.pending_voting_rewards;
    let staked_amount =
        load_token_balance(&deps.querier, &config.nebula_token, &state.contract_addr)?
            .checked_sub(total_locked_balance)?;

    a_poll.staked_amount = Some(staked_amount);

    // Update poll data
    poll_store(deps.storage).save(&poll_id.to_be_bytes(), &a_poll)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "snapshot_poll"),
        attr("poll_id", poll_id.to_string()),
        attr("staked_amount", staked_amount),
    ]))
}

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Config {}** Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// - **QueryMsg::State {}** Returns the current state of the governance contract.
///
/// - **QueryMsg::Staker { address }** Returns the specified staker information.
///
/// - **QueryMsg::Poll { poll_id }** Returns the specified poll information.
///
/// - **QueryMsg::Polls {
///             filter,
///             start_after,
///             limit,
///             order_by,
///         }** Returns a list of poll information filtered by the provided criterions.
///
/// - **QueryMsg::Voter {
///             poll_id,
///             address,
///         }** Returns the vote data on the specified poll of the given address.
///
/// - **QueryMsg::Voters {
///             poll_id,
///             start_after,
///             limit,
///             order_by,
///         }** Returns a list of voter data filtered by the provided criterions.
///
/// - **QueryMsg::Shares {
///             start_after,
///             limit,
///             order_by,
///         }** Returns a list of staker shares filtered by the provided criterions.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Staker { address } => to_binary(&query_staker(deps, address)?),
        QueryMsg::Poll { poll_id } => to_binary(&query_poll(deps, poll_id)?),
        QueryMsg::Polls {
            filter,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_polls(deps, filter, start_after, limit, order_by)?),
        QueryMsg::Voter { poll_id, address } => to_binary(&query_voter(deps, poll_id, address)?),
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
        } => to_binary(&query_shares(deps, start_after, limit, order_by)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = config_read(deps.storage).load()?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        nebula_token: config.nebula_token.to_string(),
        quorum: config.quorum,
        threshold: config.threshold,
        voting_period: config.voting_period,
        effective_delay: config.effective_delay,
        proposal_deposit: config.proposal_deposit,
        voter_weight: config.voter_weight,
        snapshot_period: config.snapshot_period,
    })
}

/// ## Description
/// Returns the current state of the governance contract containing
/// - the total number of polls
/// - the total staker share
/// - the sum initial deposit of in-progress polls
/// - the current pending rewards for voters
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state: State = state_read(deps.storage).load()?;
    Ok(StateResponse {
        poll_count: state.poll_count,
        total_share: state.total_share,
        total_deposit: state.total_deposit,
        pending_voting_rewards: state.pending_voting_rewards,
    })
}

/// ## Description
/// Returns the specified poll information.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **poll_id** is an object of type [`u64`].
fn query_poll(deps: Deps, poll_id: u64) -> StdResult<PollResponse> {
    let poll = match poll_read(deps.storage).may_load(&poll_id.to_be_bytes())? {
        Some(poll) => poll,
        None => return Err(StdError::generic_err("Poll does not exist")),
    };

    Ok(PollResponse {
        id: poll.id,
        creator: poll.creator.to_string(),
        status: poll.status,
        end_time: poll.end_time,
        title: poll.title,
        description: poll.description,
        link: poll.link,
        deposit_amount: poll.deposit_amount,
        execute_data: if let Some(execute_data) = poll.execute_data {
            Some(
                execute_data
                    .iter()
                    .map(|poll_execute_msg| -> PollExecuteMsg {
                        PollExecuteMsg {
                            contract: poll_execute_msg.contract.to_string(),
                            msg: poll_execute_msg.msg.clone(),
                        }
                    })
                    .collect(),
            )
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

/// ## Description
/// Returns a list of poll information filtered by the provided criterions.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **filter** is an object of type [`Option<PollStatus>`] which is a filter for the poll status.
///
/// - **start_after** is an object of type [`Option<u64>`] which is a filter for the poll ID.
///
/// - **limit** is an object of type [`Option<u32>`] which limits the number of polls in the query result.
///
/// - **order_by** is an object of type [`Option<OrderBy>`] which specifies the ordering of the result.
fn query_polls(
    deps: Deps,
    filter: Option<PollStatus>,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<PollsResponse> {
    // Query polls matching the provided criterions from the storage
    let polls = read_polls(deps.storage, filter, start_after, limit, order_by, None)?;
    // Extract the result into a vector of `PollResponse`
    let poll_responses: StdResult<Vec<PollResponse>> = polls
        .iter()
        .map(|poll| {
            Ok(PollResponse {
                id: poll.id,
                creator: poll.creator.to_string(),
                status: poll.status.clone(),
                end_time: poll.end_time,
                title: poll.title.to_string(),
                description: poll.description.to_string(),
                link: poll.link.clone(),
                deposit_amount: poll.deposit_amount,
                execute_data: if let Some(execute_data) = poll.execute_data.clone() {
                    Some(
                        execute_data
                            .iter()
                            .map(|poll_execute_msg| -> PollExecuteMsg {
                                PollExecuteMsg {
                                    contract: poll_execute_msg.contract.to_string(),
                                    msg: poll_execute_msg.msg.clone(),
                                }
                            })
                            .collect(),
                    )
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

/// ## Description
/// Returns the vote data on the specified poll of the given address.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **poll_id** is an object of type [`u64`].
///
/// - **address** is an object of type [`String`].
fn query_voter(deps: Deps, poll_id: u64, address: String) -> StdResult<VotersResponseItem> {
    // Query the vote information of the address from the storage
    let voter: VoterInfo = poll_voter_read(deps.storage, poll_id)
        .load(deps.api.addr_validate(address.as_str())?.as_bytes())?;
    Ok(VotersResponseItem {
        voter: address,
        vote: voter.vote,
        balance: voter.balance,
    })
}

/// ## Description
/// Returns a list of voter data filtered by the provided criterions.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **poll_id** is an object of type [`u64`].
///
/// - **start_after** is an object of type [`Option<String>`] which is a filter for voter address.
///
/// - **limit** is an object of type [`Option<u32>`] which limits the number of voters in the query result.
///
/// - **order_by** is an object of type [`Option<OrderBy>`] which specifies the ordering of the result.
fn query_voters(
    deps: Deps,
    poll_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<VotersResponse> {
    // Query voters in the specified poll matching the provided criterions from the storage
    let voters = if let Some(start_after) = start_after {
        read_poll_voters(
            deps.storage,
            poll_id,
            Some(deps.api.addr_validate(&start_after.as_str())?),
            limit,
            order_by,
        )?
    } else {
        read_poll_voters(deps.storage, poll_id, None, limit, order_by)?
    };

    // Extract the result into a vector of `VoterResponseItem`
    let voters_response: StdResult<Vec<VotersResponseItem>> = voters
        .iter()
        .map(|voter_info| {
            Ok(VotersResponseItem {
                voter: voter_info.0.to_string(),
                vote: voter_info.1.vote.clone(),
                balance: voter_info.1.balance,
            })
        })
        .collect();

    Ok(VotersResponse {
        voters: voters_response?,
    })
}
/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
