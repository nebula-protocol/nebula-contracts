use crate::contract::{handle, init, query};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::querier::load_token_balance;
use crate::staking::SECONDS_PER_WEEK;
use crate::state::{
    bank_read, bank_store, config_read, poll_indexer_store, poll_store, poll_voter_read,
    poll_voter_store, state_read, total_voting_power_read, Config, Poll, State, TokenManager,
    TotalVotingPower,
};

use cosmwasm_std::testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, log, to_binary, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HumanAddr, StdError, Uint128, WasmMsg,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, HandleMsg, InitMsg, PollResponse, PollStatus,
    PollsResponse, QueryMsg, SharesResponse, SharesResponseItem, StakerResponse, StateResponse,
    VoteOption, VoterInfo, VotersResponse, VotersResponseItem,
};

const VOTING_TOKEN: &str = "voting_token";
const TEST_CREATOR: &str = "creator";
const TEST_VOTER: &str = "voter1";
const TEST_VOTER_2: &str = "voter2";
const TEST_VOTER_3: &str = "voter3";
const TEST_COLLECTOR: &str = "collector";
const DEFAULT_QUORUM: u64 = 30u64;
const DEFAULT_THRESHOLD: u64 = 50u64;
const DEFAULT_VOTING_PERIOD: u64 = 10000u64;
const DEFAULT_EFFECTIVE_DELAY: u64 = 10000u64;
const DEFAULT_EXPIRATION_PERIOD: u64 = 20000u64;
const DEFAULT_PROPOSAL_DEPOSIT: u128 = 10000000000u128;
const DEFAULT_VOTER_WEIGHT: Decimal = Decimal::zero();
const DEFAULT_SNAPSHOT_PERIOD: u64 = 10u64;

fn mock_init(mut deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: DEFAULT_VOTER_WEIGHT,
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
}

fn mock_env_height(sender: &str, sent: &[Coin], height: u64, time: u64) -> Env {
    let mut env = mock_env(sender, sent);
    env.block.height = height;
    env.block.time = time;
    env
}

fn init_msg() -> InitMsg {
    InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: DEFAULT_VOTER_WEIGHT,
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    }
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();
    let env = mock_env(TEST_CREATOR, &coins(2, VOTING_TOKEN));
    let res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config: Config = config_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        config,
        Config {
            nebula_token: HumanAddr::from(VOTING_TOKEN),
            owner: HumanAddr::from(TEST_CREATOR),
            quorum: Decimal::percent(DEFAULT_QUORUM),
            threshold: Decimal::percent(DEFAULT_THRESHOLD),
            voting_period: DEFAULT_VOTING_PERIOD,
            effective_delay: DEFAULT_EFFECTIVE_DELAY,
            expiration_period: DEFAULT_EXPIRATION_PERIOD,
            proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            voter_weight: DEFAULT_VOTER_WEIGHT,
            snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
        }
    );

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::zero(),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

#[test]
fn poll_not_found() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 });

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Poll does not exist"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }
}

#[test]
fn fails_create_poll_invalid_quorum() {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("voter", &coins(11, VOTING_TOKEN));
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(101),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: DEFAULT_VOTER_WEIGHT,
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let res = init(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "quorum must be 0 to 1"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_create_poll_invalid_threshold() {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("voter", &coins(11, VOTING_TOKEN));
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(101),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: DEFAULT_VOTER_WEIGHT,
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let res = init(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "threshold must be 0 to 1"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_create_poll_invalid_title() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let msg = create_poll_msg("a".to_string(), "test".to_string(), None, None);
    let env = mock_env(VOTING_TOKEN, &vec![]);
    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Title too short"),
        Err(_) => panic!("Unknown error"),
    }

    let msg = create_poll_msg(
            "0123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234012345678901234567890123456789012345678901234567890123456789012340123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234".to_string(),
            "test".to_string(),
            None,
            None,
        );

    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Title too long"),
        Err(_) => panic!("Unknown error"),
    }
}

#[test]
fn fails_create_poll_invalid_description() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let msg = create_poll_msg("test".to_string(), "a".to_string(), None, None);
    let env = mock_env(VOTING_TOKEN, &vec![]);
    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Description too short"),
        Err(_) => panic!("Unknown error"),
    }

    let msg = create_poll_msg(
            "test".to_string(),
            "0123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234012345678901234567890123456789012345678901234567890123456789012340123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234".to_string(),
            None,
            None,
        );

    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Description too long"),
        Err(_) => panic!("Unknown error"),
    }
}

#[test]
fn fails_create_poll_invalid_link() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        Some("http://hih".to_string()),
        None,
    );
    let env = mock_env(VOTING_TOKEN, &vec![]);
    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Link too short"),
        Err(_) => panic!("Unknown error"),
    }

    let msg = create_poll_msg(
            "test".to_string(),
            "test".to_string(),
            Some("0123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234012345678901234567890123456789012345678901234567890123456789012340123456789012345678901234567890123456789012345678901234567890123401234567890123456789012345678901234567890123456789012345678901234".to_string()),
            None,
        );

    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Link too long"),
        Err(_) => panic!("Unknown error"),
    }
}

#[test]
fn fails_create_poll_invalid_deposit() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_CREATOR),
        amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT - 1),
        msg: Some(
            to_binary(&Cw20HookMsg::CreatePoll {
                title: "TESTTEST".to_string(),
                description: "TESTTEST".to_string(),
                link: None,
                execute_msg: None,
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN, &vec![]);
    match handle(&mut deps, env.clone(), msg) {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            format!("Must deposit more than {} token", DEFAULT_PROPOSAL_DEPOSIT)
        ),
        Err(_) => panic!("Unknown error"),
    }
}

fn create_poll_msg(
    title: String,
    description: String,
    link: Option<String>,
    execute_msg: Option<ExecuteMsg>,
) -> HandleMsg {
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_CREATOR),
        amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        msg: Some(
            to_binary(&Cw20HookMsg::CreatePoll {
                title,
                description,
                link,
                execute_msg,
            })
            .unwrap(),
        ),
    });
    msg
}

#[test]
fn happy_days_create_poll() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );
}

#[test]
fn query_polls() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);

    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        Some("http://google.com".to_string()),
        None,
    );
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    let msg = create_poll_msg("test2".to_string(), "test2".to_string(), None, None);
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: None,
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.polls,
        vec![
            PollResponse {
                id: 1u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::InProgress,
                end_height: 10000u64,
                title: "test".to_string(),
                description: "test".to_string(),
                link: Some("http://google.com".to_string()),
                deposit_amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
                execute_data: None,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                staked_amount: None,
            },
            PollResponse {
                id: 2u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::InProgress,
                end_height: 10000u64,
                title: "test2".to_string(),
                description: "test2".to_string(),
                link: None,
                deposit_amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
                execute_data: None,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                staked_amount: None,
            },
        ]
    );

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: None,
            start_after: Some(1u64),
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.polls,
        vec![PollResponse {
            id: 2u64,
            creator: HumanAddr::from(TEST_CREATOR),
            status: PollStatus::InProgress,
            end_height: 10000u64,
            title: "test2".to_string(),
            description: "test2".to_string(),
            link: None,
            deposit_amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            execute_data: None,
            yes_votes: Uint128::zero(),
            no_votes: Uint128::zero(),
            total_balance_at_end_poll: None,
            voters_reward: Uint128::zero(),
            abstain_votes: Uint128::zero(),
            staked_amount: None,
        },]
    );

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: None,
            start_after: Some(2u64),
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.polls,
        vec![PollResponse {
            id: 1u64,
            creator: HumanAddr::from(TEST_CREATOR),
            status: PollStatus::InProgress,
            end_height: 10000u64,
            title: "test".to_string(),
            description: "test".to_string(),
            link: Some("http://google.com".to_string()),
            deposit_amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            execute_data: None,
            yes_votes: Uint128::zero(),
            no_votes: Uint128::zero(),
            total_balance_at_end_poll: None,
            voters_reward: Uint128::zero(),
            abstain_votes: Uint128::zero(),
            staked_amount: None,
        }]
    );

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::InProgress),
            start_after: Some(1u64),
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.polls,
        vec![PollResponse {
            id: 2u64,
            creator: HumanAddr::from(TEST_CREATOR),
            status: PollStatus::InProgress,
            end_height: 10000u64,
            title: "test2".to_string(),
            description: "test2".to_string(),
            link: None,
            deposit_amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            execute_data: None,
            yes_votes: Uint128::zero(),
            no_votes: Uint128::zero(),
            total_balance_at_end_poll: None,
            voters_reward: Uint128::zero(),
            abstain_votes: Uint128::zero(),
            staked_amount: None,
        },]
    );

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Passed),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls, vec![]);
}

#[test]
fn create_poll_no_quorum() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );
}

#[test]
fn fails_end_poll_before_end_height() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(DEFAULT_VOTING_PERIOD, value.end_height);

    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let env = mock_env_height(TEST_CREATOR, &vec![], 0, 10000);
    let handle_res = handle(&mut deps, env, msg);

    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Voting period has not expired"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn happy_days_end_poll() {
    const POLL_START_HEIGHT: u64 = 1000;
    const POLL_ID: u64 = 1;
    let stake_amount = 1000;

    let mut deps = mock_dependencies(20, &coins(1000, VOTING_TOKEN));
    mock_init(&mut deps);
    let mut creator_env = mock_env_height(
        VOTING_TOKEN,
        &coins(2, VOTING_TOKEN),
        POLL_START_HEIGHT,
        10000,
    );

    let exec_msg_bz = to_binary(&Cw20HandleMsg::Burn {
        amount: Uint128(123),
    })
    .unwrap();
    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        None,
        Some(ExecuteMsg {
            contract: HumanAddr::from(VOTING_TOKEN),
            msg: exec_msg_bz.clone(),
        }),
    );

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_create_poll_result(
        1,
        creator_env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let mut env = mock_env_height(TEST_VOTER, &[], POLL_START_HEIGHT, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "1000"),
            log("voter", TEST_VOTER),
            log("vote_option", "yes"),
        ]
    );

    // not in passed status
    let msg = HandleMsg::ExecutePoll { poll_id: 1 };

    env.block.height = creator_env.block.height;

    let handle_res = handle(&mut deps, env.clone(), msg).unwrap_err();
    match handle_res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "Poll is not in passed status"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD;

    env.block.height = creator_env.block.height;

    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "1"),
            log("tallied_weight", "1000"),
            log("staked_weight", "1000"),
            log("poll_id", "1"),
            log("rejected_reason", ""),
            log("passed", "true"),
        ]
    );
    assert_eq!(
        handle_res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_CREATOR),
                amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            })
            .unwrap(),
            send: vec![],
        })]
    );

    // End poll will withdraw deposit balance
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(stake_amount as u128),
        )],
    )]);

    // effective delay has not expired
    let msg = HandleMsg::ExecutePoll { poll_id: 1 };

    env.block.height = creator_env.block.height;

    let handle_res = handle(&mut deps, env.clone(), msg).unwrap_err();
    match handle_res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "Effective delay has not expired"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    creator_env.block.height = &creator_env.block.height + DEFAULT_EFFECTIVE_DELAY;
    env.block.height = creator_env.block.height;

    let msg = HandleMsg::ExecutePoll { poll_id: 1 };

    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        handle_res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: exec_msg_bz,
            send: vec![],
        }),]
    );
    assert_eq!(
        handle_res.log,
        vec![log("action", "execute_poll"), log("poll_id", "1"),]
    );

    // Query executed polls
    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Passed),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 0);

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::InProgress),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 0);

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Executed),
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 1);

    // voter info must be deleted
    let res = query(
        &deps,
        QueryMsg::Voters {
            poll_id: 1u64,
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: VotersResponse = from_binary(&res).unwrap();
    assert_eq!(response.voters.len(), 0);

    // staker locked token must disappeared
    let res = query(
        &deps,
        QueryMsg::Staker {
            address: HumanAddr::from(TEST_VOTER),
        },
    )
    .unwrap();
    let response: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(
        response,
        StakerResponse {
            balance: Uint128(stake_amount),
            share: Uint128(stake_amount),
            locked_balance: vec![],
            pending_voting_rewards: Uint128::zero(),
            lock_end_week: Some(env.block.time / SECONDS_PER_WEEK + 104),
        }
    );
}

#[test]
fn expire_poll() {
    const POLL_START_HEIGHT: u64 = 1000;
    const POLL_ID: u64 = 1;
    let stake_amount = 1000;

    let mut deps = mock_dependencies(20, &coins(1000, VOTING_TOKEN));
    mock_init(&mut deps);
    let mut creator_env = mock_env_height(
        VOTING_TOKEN,
        &coins(2, VOTING_TOKEN),
        POLL_START_HEIGHT,
        10000,
    );

    let exec_msg_bz = to_binary(&Cw20HandleMsg::Burn {
        amount: Uint128(123),
    })
    .unwrap();
    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        None,
        Some(ExecuteMsg {
            contract: HumanAddr::from(VOTING_TOKEN),
            msg: exec_msg_bz.clone(),
        }),
    );

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_create_poll_result(
        1,
        creator_env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env_height(TEST_VOTER, &[], POLL_START_HEIGHT, env.block.time);
    let handle_res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "1000"),
            log("voter", TEST_VOTER),
            log("vote_option", "yes"),
        ]
    );

    // Poll is not in passed status
    creator_env.block.height = &creator_env.block.height + DEFAULT_EFFECTIVE_DELAY;
    let msg = HandleMsg::ExpirePoll { poll_id: 1 };
    let handle_res = handle(&mut deps, creator_env.clone(), msg);
    match handle_res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Poll is not in passed status"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "1"),
            log("tallied_weight", "1000"),
            log("staked_weight", "1000"),
            log("poll_id", "1"),
            log("rejected_reason", ""),
            log("passed", "true"),
        ]
    );
    assert_eq!(
        handle_res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_CREATOR),
                amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            })
            .unwrap(),
            send: vec![],
        })]
    );

    // Expiration period has not been passed
    let msg = HandleMsg::ExpirePoll { poll_id: 1 };
    let handle_res = handle(&mut deps, creator_env.clone(), msg);
    match handle_res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Expire height has not been reached")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    creator_env.block.height = &creator_env.block.height + DEFAULT_EXPIRATION_PERIOD;
    let msg = HandleMsg::ExpirePoll { poll_id: 1 };
    let _handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let poll_res: PollResponse = from_binary(&res).unwrap();
    assert_eq!(poll_res.status, PollStatus::Expired);

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Expired),
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let polls_res: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(polls_res.polls[0], poll_res);
}

#[test]
fn end_poll_zero_quorum() {
    let mut deps = mock_dependencies(20, &coins(1000, VOTING_TOKEN));
    mock_init(&mut deps);
    let mut creator_env = mock_env_height(VOTING_TOKEN, &vec![], 1000, 10000);

    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        None,
        Some(ExecuteMsg {
            contract: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Burn {
                amount: Uint128(123),
            })
            .unwrap(),
        }),
    );

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();
    assert_create_poll_result(
        1,
        creator_env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );
    let stake_amount = 100;
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(100u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    handle(&mut deps, env, msg.clone()).unwrap();

    let msg = HandleMsg::EndPoll { poll_id: 1 };
    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD;

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0"),
            log("tallied_weight", "0"),
            log("staked_weight", "0"),
            log("poll_id", "1"),
            log("rejected_reason", "Quorum not reached"),
            log("passed", "false"),
        ]
    );

    assert_eq!(handle_res.messages.len(), 0usize);

    // Query rejected polls
    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Rejected),
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 1);

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::InProgress),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 0);

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: Some(PollStatus::Passed),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(response.polls.len(), 0);
}

#[test]
fn end_poll_quorum_rejected() {
    let mut deps = mock_dependencies(20, &coins(100, VOTING_TOKEN));
    mock_init(&mut deps);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let mut creator_env = mock_env(VOTING_TOKEN, &vec![]);
    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "create_poll"),
            log("creator", TEST_CREATOR),
            log("poll_id", "1"),
            log("end_height", "22345"),
        ]
    );

    let stake_amount = 100;
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(100u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(10u128),
    };
    let env = mock_env(TEST_VOTER, &[]);
    let handle_res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", "1"),
            log("amount", "10"),
            log("voter", TEST_VOTER),
            log("vote_option", "yes"),
        ]
    );

    let msg = HandleMsg::EndPoll { poll_id: 1 };

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD;

    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0.1"),
            log("tallied_weight", "10"),
            log("staked_weight", "100"),
            log("poll_id", "1"),
            log("rejected_reason", "Quorum not reached"),
            log("passed", "false"),
        ]
    );
}

#[test]
fn end_poll_quorum_rejected_noting_staked() {
    let mut deps = mock_dependencies(20, &coins(100, VOTING_TOKEN));
    mock_init(&mut deps);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let mut creator_env = mock_env(VOTING_TOKEN, &vec![]);
    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "create_poll"),
            log("creator", TEST_CREATOR),
            log("poll_id", "1"),
            log("end_height", "22345"),
        ]
    );

    let msg = HandleMsg::EndPoll { poll_id: 1 };

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD;

    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0"),
            log("tallied_weight", "0"),
            log("staked_weight", "0"),
            log("poll_id", "1"),
            log("rejected_reason", "Quorum not reached"),
            log("passed", "false"),
        ]
    );
}

#[test]
fn end_poll_nay_rejected() {
    let voter1_stake = 100;
    let voter2_stake = 1000;
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let mut creator_env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "create_poll"),
            log("creator", TEST_CREATOR),
            log("poll_id", "1"),
            log("end_height", "22345"),
        ]
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((voter1_stake + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(voter1_stake as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg).unwrap();
    assert_stake_tokens_result(
        voter1_stake,
        DEFAULT_PROPOSAL_DEPOSIT,
        voter1_stake,
        1,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((voter1_stake + voter2_stake + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_2),
        amount: Uint128::from(voter2_stake as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg).unwrap();
    assert_stake_tokens_result(
        voter1_stake + voter2_stake,
        DEFAULT_PROPOSAL_DEPOSIT,
        voter2_stake,
        1,
        handle_res,
        &mut deps,
    );

    let env = mock_env(TEST_VOTER_2, &[]);
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::No,
        amount: Uint128::from(voter2_stake),
    };
    let handle_res = handle(&mut deps, env, msg).unwrap();
    assert_cast_vote_success(TEST_VOTER_2, voter2_stake, 1, VoteOption::No, handle_res);

    let msg = HandleMsg::EndPoll { poll_id: 1 };

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD;
    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0.90909090909090909"),
            log("tallied_weight", "1000"),
            log("staked_weight", "1100"),
            log("poll_id", "1"),
            log("rejected_reason", "Threshold not reached"),
            log("passed", "false"),
        ]
    );
}

#[test]
fn fails_cast_vote_not_enough_staked() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(10u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(10u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(10, DEFAULT_PROPOSAL_DEPOSIT, 10, 1, handle_res, &mut deps);

    let env = mock_env_height(TEST_VOTER, &coins(11, VOTING_TOKEN), 0, env.block.time);
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(11u128),
    };

    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "User does not have enough staked tokens.")
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn happy_days_cast_vote() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(11u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let lock_for_weeks = 104u64;
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(lock_for_weeks),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(11, DEFAULT_PROPOSAL_DEPOSIT, 11, 1, handle_res, &mut deps);

    let env = mock_env_height(TEST_VOTER, &coins(11, VOTING_TOKEN), 0, env.block.time);
    let amount = 10u128;
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(amount),
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_cast_vote_success(TEST_VOTER, amount, 1, VoteOption::Yes, handle_res);

    // balance be double
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(22u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    // Query staker
    let res = query(
        &deps,
        QueryMsg::Staker {
            address: HumanAddr::from(TEST_VOTER),
        },
    )
    .unwrap();
    let response: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(
        response,
        StakerResponse {
            balance: Uint128(22u128),
            share: Uint128(11u128),
            locked_balance: vec![(
                1u64,
                VoterInfo {
                    vote: VoteOption::Yes,
                    balance: Uint128::from(amount),
                }
            )],
            pending_voting_rewards: Uint128::zero(),
            lock_end_week: Some(env.block.time / SECONDS_PER_WEEK + lock_for_weeks),
        }
    );

    // Query voters
    let res = query(
        &deps,
        QueryMsg::Voters {
            poll_id: 1u64,
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let response: VotersResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.voters,
        vec![VotersResponseItem {
            voter: HumanAddr::from(TEST_VOTER),
            vote: VoteOption::Yes,
            balance: Uint128::from(amount),
        }]
    );

    let res = query(
        &deps,
        QueryMsg::Voters {
            poll_id: 1u64,
            start_after: Some(HumanAddr::from(TEST_VOTER)),
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let response: VotersResponse = from_binary(&res).unwrap();
    assert_eq!(response.voters.len(), 0);
}

#[test]
fn happy_days_withdraw_voting_tokens() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(11u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(11, 0, 11, 0, handle_res, &mut deps);

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::from(11u128),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );

    // double the balance, only half will be withdrawn
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(22u128))],
    )]);

    let env = mock_env(TEST_VOTER, &[]);
    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(11u128)),
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone());

    // Should not allow withdrawal of tokens before lock up expiry.
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "User is trying to withdraw tokens before expiry.")
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let env = mock_env_height(TEST_VOTER, &[], 0, env.block.time + 104 * SECONDS_PER_WEEK);

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();

    let msg = handle_res.messages.get(0).expect("no message");

    assert_eq!(
        msg,
        &CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_VOTER),
                amount: Uint128::from(11u128),
            })
            .unwrap(),
            send: vec![],
        })
    );

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::from(6u128),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

#[test]
fn happy_days_withdraw_voting_tokens_all() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(11u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(11, 0, 11, 0, handle_res, &mut deps);

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::from(11u128),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );

    // double the balance, all balance withdrawn
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(22u128))],
    )]);

    let mut env = mock_env(TEST_VOTER, &[]);

    env.block.time += 104 * SECONDS_PER_WEEK;

    let msg = HandleMsg::WithdrawVotingTokens { amount: None };

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    let msg = handle_res.messages.get(0).expect("no message");

    assert_eq!(
        msg,
        &CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_VOTER),
                amount: Uint128::from(22u128),
            })
            .unwrap(),
            send: vec![],
        })
    );

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::zero(),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

#[test]
fn withdraw_voting_tokens() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(11u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(11, 0, 11, 0, handle_res, &mut deps);

    // make fake polls; one in progress & one in passed
    poll_store(&mut deps.storage)
        .save(
            &1u64.to_be_bytes(),
            &Poll {
                id: 1u64,
                creator: HumanAddr::default(),
                status: PollStatus::InProgress,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                staked_amount: None,
                max_voting_power: Uint128::zero(),
            },
        )
        .unwrap();

    poll_store(&mut deps.storage)
        .save(
            &2u64.to_be_bytes(),
            &Poll {
                id: 1u64,
                creator: HumanAddr::default(),
                status: PollStatus::Passed,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                staked_amount: None,
                max_voting_power: Uint128::zero(),
            },
        )
        .unwrap();

    let voter_addr = HumanAddr::from(TEST_VOTER);
    poll_voter_store(&mut deps.storage, 1u64)
        .save(
            &voter_addr.as_str().as_bytes(),
            &VoterInfo {
                vote: VoteOption::Yes,
                balance: Uint128(5u128),
            },
        )
        .unwrap();
    poll_voter_store(&mut deps.storage, 2u64)
        .save(
            &voter_addr.as_str().as_bytes(),
            &VoterInfo {
                vote: VoteOption::Yes,
                balance: Uint128(5u128),
            },
        )
        .unwrap();
    bank_store(&mut deps.storage)
        .save(
            &voter_addr.as_str().as_bytes(),
            &TokenManager {
                share: Uint128(11u128),
                locked_balance: vec![
                    (
                        1u64,
                        VoterInfo {
                            vote: VoteOption::Yes,
                            balance: Uint128(5u128),
                        },
                    ),
                    (
                        2u64,
                        VoterInfo {
                            vote: VoteOption::Yes,
                            balance: Uint128(5u128),
                        },
                    ),
                ],
                participated_polls: vec![],
                lock_end_week: Some(env.block.time / SECONDS_PER_WEEK),
            },
        )
        .unwrap();

    let mut env = mock_env(TEST_VOTER, &[]);

    env.block.time += 104 * SECONDS_PER_WEEK;
    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(5u128)),
    };

    let _ = handle(&mut deps, env, msg).unwrap();
    let voter = poll_voter_read(&deps.storage, 1u64)
        .load(&voter_addr.as_str().as_bytes())
        .unwrap();
    assert_eq!(
        voter,
        VoterInfo {
            vote: VoteOption::Yes,
            balance: Uint128(5u128),
        }
    );

    let token_manager = bank_read(&deps.storage)
        .load(&voter_addr.as_str().as_bytes())
        .unwrap();
    assert_eq!(
        token_manager.locked_balance,
        vec![(
            1u64,
            VoterInfo {
                vote: VoteOption::Yes,
                balance: Uint128(5u128),
            }
        )]
    );
}

#[test]
fn fails_withdraw_voting_tokens_no_stake() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let env = mock_env(TEST_VOTER, &coins(11, VOTING_TOKEN));
    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(11u128)),
    };

    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Nothing staked"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_withdraw_too_many_tokens() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(10u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(10u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(10, 0, 10, 0, handle_res, &mut deps);

    let mut env = mock_env(TEST_VOTER, &[]);
    env.block.time += 104 * SECONDS_PER_WEEK;
    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(11u128)),
    };

    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "User is trying to withdraw too many tokens.")
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_cast_vote_twice() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let env = mock_env_height(VOTING_TOKEN, &coins(2, VOTING_TOKEN), 0, 10000);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_create_poll_result(
        1,
        env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(11u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(11, DEFAULT_PROPOSAL_DEPOSIT, 11, 1, handle_res, &mut deps);

    let amount = 1u128;
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(amount),
    };
    let env = mock_env_height(TEST_VOTER, &[], 0, 10000);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_cast_vote_success(TEST_VOTER, amount, 1, VoteOption::Yes, handle_res);

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(amount),
    };
    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "User has already voted."),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_cast_vote_without_poll() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let msg = HandleMsg::CastVote {
        poll_id: 0,
        vote: VoteOption::Yes,
        amount: Uint128::from(1u128),
    };
    let env = mock_env(TEST_VOTER, &coins(11, VOTING_TOKEN));

    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Poll does not exist"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn happy_days_stake_voting_tokens() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(11u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(11, 0, 11, 0, handle_res, &mut deps);
}

#[test]
fn fails_insufficient_funds() {
    let mut deps = mock_dependencies(20, &[]);

    // initialize the store
    let msg = init_msg();
    let env = mock_env(TEST_VOTER, &coins(2, VOTING_TOKEN));
    let init_res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    // insufficient token
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(0u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Insufficient funds sent"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn fails_staking_wrong_token() {
    let mut deps = mock_dependencies(20, &[]);

    // initialize the store
    let msg = init_msg();
    let env = mock_env(TEST_VOTER, &coins(2, VOTING_TOKEN));
    let init_res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(11u128))],
    )]);

    // wrong token
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN.to_string() + "2", &[]);
    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::Unauthorized { .. }) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn share_calculation() {
    let mut deps = mock_dependencies(20, &[]);

    // initialize the store
    let msg = init_msg();
    let env = mock_env(TEST_VOTER, &coins(2, VOTING_TOKEN));
    let init_res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    // create 100 share
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(100u128))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(100u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env, msg);

    // add more balance(100) to make share:balance = 1:2
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(200u128 + 100u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(100u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: None,
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "staking"),
            log("sender", TEST_VOTER),
            log("share", "50"),
            log("amount", "100"),
        ]
    );

    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128(100u128)),
    };
    let mut env = mock_env(TEST_VOTER.to_string(), &[]);
    env.block.time += 104 * SECONDS_PER_WEEK;
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("recipient", TEST_VOTER),
            log("amount", "100"),
        ]
    );

    // 100 tokens withdrawn
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(200u128))],
    )]);

    let res = query(
        &mut deps,
        QueryMsg::Staker {
            address: HumanAddr::from(TEST_VOTER),
        },
    )
    .unwrap();
    let stake_info: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(stake_info.share, Uint128(100));
    assert_eq!(stake_info.balance, Uint128(200));
    assert_eq!(stake_info.locked_balance, vec![]);
}

#[test]
fn share_calculation_with_voter_rewards() {
    let mut deps = mock_dependencies(20, &[]);

    // initialize the store
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };
    let env = mock_env(TEST_VOTER, &coins(2, VOTING_TOKEN));
    let init_res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, init_res.messages.len());

    // create poll
    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    // create 100 share
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(DEFAULT_PROPOSAL_DEPOSIT + 100u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(100u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "staking"),
            log("sender", TEST_VOTER),
            log("share", "100"),
            log("amount", "100"),
        ]
    );

    // add more balance through dept reward, 50% reserved for voters
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(DEFAULT_PROPOSAL_DEPOSIT + 400u128 + 100u128),
        )],
    )]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(400u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(DEFAULT_PROPOSAL_DEPOSIT + 400u128 + 100u128),
        )],
    )]);
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(100u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: None,
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "staking"),
            log("sender", TEST_VOTER),
            log("share", "50"),
            log("amount", "100"),
        ]
    );

    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128(100u128)),
    };
    let mut env = mock_env(TEST_VOTER.to_string(), &[]);
    env.block.time += 104 * SECONDS_PER_WEEK;
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("recipient", TEST_VOTER),
            log("amount", "100"),
        ]
    );

    // 100 tokens withdrawn
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(DEFAULT_PROPOSAL_DEPOSIT + 400u128),
        )],
    )]);

    let res = query(
        &mut deps,
        QueryMsg::Staker {
            address: HumanAddr::from(TEST_VOTER),
        },
    )
    .unwrap();
    let stake_info: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(stake_info.share, Uint128(100));
    assert_eq!(stake_info.balance, Uint128(200));
    assert_eq!(stake_info.locked_balance, vec![]);
}

// helper to confirm the expected create_poll response
fn assert_create_poll_result(
    poll_id: u64,
    end_height: u64,
    creator: &str,
    handle_res: HandleResponse,
    deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>,
) {
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "create_poll"),
            log("creator", creator),
            log("poll_id", poll_id.to_string()),
            log("end_height", end_height.to_string()),
        ]
    );

    //confirm poll count
    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 1,
            total_share: Uint128::zero(),
            total_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

fn assert_stake_tokens_result(
    total_share: u128,
    total_deposit: u128,
    new_share: u128,
    poll_count: u64,
    handle_res: HandleResponse,
    deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>,
) {
    assert_eq!(
        handle_res.log.get(2).expect("no log"),
        &log("share", new_share.to_string())
    );

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count,
            total_share: Uint128(total_share),
            total_deposit: Uint128(total_deposit),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

fn assert_cast_vote_success(
    voter: &str,
    amount: u128,
    poll_id: u64,
    vote_option: VoteOption,
    handle_res: HandleResponse,
) {
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", poll_id.to_string()),
            log("amount", amount.to_string()),
            log("voter", voter),
            log("vote_option", vote_option.to_string()),
        ]
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    // update owner
    let env = mock_env(TEST_CREATOR, &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr("addr0001".to_string())),
        quorum: None,
        threshold: None,
        voting_period: None,
        effective_delay: None,
        expiration_period: None,
        proposal_deposit: None,
        voter_weight: None,
        snapshot_period: None,
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("addr0001", config.owner.as_str());
    assert_eq!(Decimal::percent(DEFAULT_QUORUM), config.quorum);
    assert_eq!(Decimal::percent(DEFAULT_THRESHOLD), config.threshold);
    assert_eq!(DEFAULT_VOTING_PERIOD, config.voting_period);
    assert_eq!(DEFAULT_EFFECTIVE_DELAY, config.effective_delay);
    assert_eq!(DEFAULT_PROPOSAL_DEPOSIT, config.proposal_deposit.u128());

    // update left items
    let env = mock_env("addr0001", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        quorum: Some(Decimal::percent(20)),
        threshold: Some(Decimal::percent(75)),
        voting_period: Some(20000u64),
        effective_delay: Some(20000u64),
        expiration_period: Some(30000u64),
        proposal_deposit: Some(Uint128(123u128)),
        voter_weight: Some(Decimal::percent(1)),
        snapshot_period: Some(60u64),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("addr0001", config.owner.as_str());
    assert_eq!(Decimal::percent(20), config.quorum);
    assert_eq!(Decimal::percent(75), config.threshold);
    assert_eq!(20000u64, config.voting_period);
    assert_eq!(20000u64, config.effective_delay);
    assert_eq!(30000u64, config.expiration_period);
    assert_eq!(123u128, config.proposal_deposit.u128());
    assert_eq!(Decimal::percent(1), config.voter_weight);
    assert_eq!(60u64, config.snapshot_period);

    // Unauthorzied err
    let env = mock_env(TEST_CREATOR, &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        quorum: None,
        threshold: None,
        voting_period: None,
        effective_delay: None,
        expiration_period: None,
        proposal_deposit: None,
        voter_weight: None,
        snapshot_period: None,
    };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn distribute_voting_rewards() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_create_poll_result(
        1,
        env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    let stake_amount = 100u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env(TEST_VOTER, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT + 100u128) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // FAIL - there is no finished polls, amount to withdraw is 0, returning error
    let msg = HandleMsg::WithdrawVotingRewards {};
    let mut env = mock_env(TEST_VOTER, &[]);
    env.block.time += 104 * SECONDS_PER_WEEK;
    let res = handle(&mut deps, env.clone(), msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("Nothing to withdraw"));

    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // SUCCESS
    let msg = HandleMsg::WithdrawVotingRewards {};
    let env = mock_env_height(TEST_VOTER, &[], 0, env.block.time);
    let res = handle(&mut deps, env.clone(), msg).unwrap();

    // user can withdraw 50% of total staked (weight = 50% poll share = 100%)
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", TEST_VOTER),
            log("amount", 50),
        ]
    );
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_VOTER),
                amount: Uint128::from(50u128),
            })
            .unwrap(),
            send: vec![],
        })]
    );

    // voting info has been deleted
    assert_eq!(
        poll_voter_read(&deps.storage, 1u64)
            .load(&HumanAddr::from(TEST_VOTER).as_str().as_bytes())
            .is_err(),
        true
    );
}

#[test]
fn distribute_voting_rewards_with_multiple_active_polls_and_voters() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };
    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    // create polls
    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    // poll 1
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    // poll 2
    let msg = create_poll_msg("test2".to_string(), "test2".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    const ALICE: &str = "alice";
    const ALICE_STAKE: u128 = 750_000_000u128;
    const BOB: &str = "bob";
    const BOB_STAKE: u128 = 250_000_000u128;
    const CINDY: &str = "cindy";
    const CINDY_STAKE: u128 = 250_000_000u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + DEFAULT_PROPOSAL_DEPOSIT * 2) as u128),
        )],
    )]);
    // Alice stakes 750 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(ALICE),
        amount: Uint128::from(ALICE_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + DEFAULT_PROPOSAL_DEPOSIT * 2) as u128),
        )],
    )]);
    // Bob stakes 250 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BOB),
        amount: Uint128::from(BOB_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(
                (ALICE_STAKE + BOB_STAKE + CINDY_STAKE + DEFAULT_PROPOSAL_DEPOSIT * 2) as u128,
            ),
        )],
    )]);
    // Cindy stakes 250 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(CINDY),
        amount: Uint128::from(CINDY_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // Alice votes on proposal 1
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(ALICE_STAKE),
    };
    let env = mock_env(ALICE, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    // Bob votes on proposals 1 and 2
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Abstain,
        amount: Uint128::from(BOB_STAKE),
    };
    let env = mock_env(BOB, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::CastVote {
        poll_id: 2,
        vote: VoteOption::No,
        amount: Uint128::from(BOB_STAKE),
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    // Cindy votes on proposal 2
    let msg = HandleMsg::CastVote {
        poll_id: 2,
        vote: VoteOption::Abstain,
        amount: Uint128::from(CINDY_STAKE),
    };
    let env = mock_env(CINDY, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(
                (ALICE_STAKE
                    + BOB_STAKE
                    + CINDY_STAKE
                    + DEFAULT_PROPOSAL_DEPOSIT * 2
                    + 2000000000u128) as u128,
            ),
        )],
    )]);

    // Collector sends 2000 NEB with 50% voting weight
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(2000000000u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // End the polls
    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    let msg = HandleMsg::EndPoll { poll_id: 2 };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::WithdrawVotingRewards {};
    // ALICE withdraws voting rewards
    let env = mock_env_height(ALICE, &[], 0, env.block.time);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", ALICE),
            log("amount", 375000000),
        ]
    );

    // BOB withdraws voting rewards
    let env = mock_env_height(BOB, &[], 0, env.block.time);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", BOB),
            log("amount", 375000000), // 125 from poll 1 + 250 from poll 2
        ]
    );

    // CINDY
    let env = mock_env_height(CINDY, &[], 0, env.block.time);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", CINDY),
            log("amount", 250000000),
        ]
    );
}

#[test]
fn distribute_voting_rewards_only_to_polls_in_progress() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };
    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    // make fake polls; one in progress & one in passed
    poll_store(&mut deps.storage)
        .save(
            &1u64.to_be_bytes(),
            &Poll {
                id: 1u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::InProgress,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                staked_amount: None,
                max_voting_power: Uint128::zero(),
            },
        )
        .unwrap();

    poll_store(&mut deps.storage)
        .save(
            &2u64.to_be_bytes(),
            &Poll {
                id: 2u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::Passed,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                staked_amount: None,
                max_voting_power: Uint128::zero(),
            },
        )
        .unwrap();

    poll_indexer_store(&mut deps.storage, &PollStatus::InProgress)
        .save(&1u64.to_be_bytes(), &true)
        .unwrap();
    poll_indexer_store(&mut deps.storage, &PollStatus::Passed)
        .save(&2u64.to_be_bytes(), &true)
        .unwrap();

    // Collector sends 2000 NEB with 50% voting weight
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(2000000000u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let res = query(
        &deps,
        QueryMsg::Polls {
            filter: None,
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: PollsResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.polls,
        vec![
            PollResponse {
                id: 1u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::InProgress,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::from(1000000000u128),
                staked_amount: None,
            },
            PollResponse {
                id: 2u64,
                creator: HumanAddr::from(TEST_CREATOR),
                status: PollStatus::Passed,
                yes_votes: Uint128::zero(),
                no_votes: Uint128::zero(),
                abstain_votes: Uint128::zero(),
                end_height: 0u64,
                title: "title".to_string(),
                description: "description".to_string(),
                deposit_amount: Uint128::zero(),
                link: None,
                execute_data: None,
                total_balance_at_end_poll: None,
                voters_reward: Uint128::zero(),
                staked_amount: None,
            },
        ]
    );
}

#[test]
fn test_staking_and_voting_rewards() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };
    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    // poll 1
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    const ALICE: &str = "alice";
    const ALICE_STAKE: u128 = 750_000_000u128;
    const BOB: &str = "bob";
    const BOB_STAKE: u128 = 250_000_000u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Alice stakes 750 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(ALICE),
        amount: Uint128::from(ALICE_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Bob stakes 250 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BOB),
        amount: Uint128::from(BOB_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // Alice votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(ALICE_STAKE),
    };
    let env = mock_env(ALICE, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    // Bob votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Abstain,
        amount: Uint128::from(BOB_STAKE),
    };
    let env = mock_env(BOB, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(
                (ALICE_STAKE + BOB_STAKE + DEFAULT_PROPOSAL_DEPOSIT + 2_000_000_000u128) as u128,
            ),
        )],
    )]);

    // Collector sends 2000 NEB with 50% voting weight
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(2_000_000_000u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });

    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // End the poll
    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // deposit is returned to creator and collector deposit is added
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + 2_000_000_000) as u128),
        )],
    )]);

    let res = query(&deps, QueryMsg::State {}).unwrap();
    let response: StateResponse = from_binary(&res).unwrap();
    assert_eq!(response.total_share, Uint128(1_000_000_000u128));
    assert_eq!(response.total_deposit, Uint128::zero());
    assert_eq!(response.pending_voting_rewards, Uint128(1_000_000_000u128));

    let res = query(
        &deps,
        QueryMsg::Staker {
            address: HumanAddr::from(ALICE),
        },
    )
    .unwrap();
    let response: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(
        response,
        StakerResponse {
            balance: Uint128(ALICE_STAKE + 750_000_000u128),
            share: Uint128(ALICE_STAKE),
            locked_balance: vec![],
            pending_voting_rewards: Uint128(750_000_000u128),
            lock_end_week: Some(env.block.time / SECONDS_PER_WEEK + 104),
        }
    );
    let res = query(
        &deps,
        QueryMsg::Staker {
            address: HumanAddr::from(BOB),
        },
    )
    .unwrap();
    let response: StakerResponse = from_binary(&res).unwrap();
    assert_eq!(
        response,
        StakerResponse {
            balance: Uint128(BOB_STAKE + 250_000_000u128),
            share: Uint128(BOB_STAKE),
            locked_balance: vec![],
            pending_voting_rewards: Uint128(250_000_000u128),
            lock_end_week: Some(env.block.time / SECONDS_PER_WEEK + 104),
        }
    );

    let msg = HandleMsg::WithdrawVotingRewards {};
    // ALICE withdraws voting rewards
    let env = mock_env(ALICE, &[]);
    let res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", ALICE),
            log("amount", ALICE_STAKE),
        ]
    );

    // BOB withdraws voting rewards
    let env = mock_env(BOB, &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", BOB),
            log("amount", BOB_STAKE),
        ]
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + 1_000_000_000) as u128),
        )],
    )]);

    // withdraw remaining voting tokens
    let msg = HandleMsg::WithdrawVotingTokens { amount: None };

    let lock_expiry_duration = 104 * SECONDS_PER_WEEK;
    let env = mock_env_height(
        ALICE,
        &[],
        env.block.height,
        env.block.time + lock_expiry_duration,
    );

    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("recipient", ALICE),
            log("amount", "1500000000"),
        ]
    );
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((BOB_STAKE + 250_000_000) as u128),
        )],
    )]);
    // withdraw remaining voting tokens
    let msg = HandleMsg::WithdrawVotingTokens { amount: None };
    let env = mock_env_height(
        BOB,
        &[],
        env.block.height,
        env.block.time + lock_expiry_duration,
    );
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("recipient", BOB),
            log("amount", "500000000"),
        ]
    );
}

#[test]
fn test_abstain_votes_theshold() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    const ALICE: &str = "alice";
    const ALICE_STAKE: u128 = 750_000_000u128;
    const BOB: &str = "bob";
    const BOB_STAKE: u128 = 250_000_000u128;
    const CINDY: &str = "cindy";
    const CINDY_STAKE: u128 = 260_000_000u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Alice stakes 750 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(ALICE),
        amount: Uint128::from(ALICE_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Bob stakes 250 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BOB),
        amount: Uint128::from(BOB_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + CINDY_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Cindy stakes 260 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(CINDY),
        amount: Uint128::from(CINDY_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // Alice votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Abstain,
        amount: Uint128::from(ALICE_STAKE),
    };
    let env = mock_env(ALICE, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    // Bob votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::No,
        amount: Uint128::from(BOB_STAKE),
    };
    let env = mock_env(BOB, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    // Cindy votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(CINDY_STAKE),
    };
    let env = mock_env(CINDY, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::EndPoll { poll_id: 1 };

    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    // abstain votes should not affect threshold, so poll is passed
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "1"),
            log("tallied_weight", "1260000000"),
            log("staked_weight", "1260000000"),
            log("poll_id", "1"),
            log("rejected_reason", ""),
            log("passed", "true"),
        ]
    );
}

#[test]
fn test_abstain_votes_quorum() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    const ALICE: &str = "alice";
    const ALICE_STAKE: u128 = 750_000_000u128;
    const BOB: &str = "bob";
    const BOB_STAKE: u128 = 50_000_000u128;
    const CINDY: &str = "cindy";
    const CINDY_STAKE: u128 = 20_000_000u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Alice stakes 750 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(ALICE),
        amount: Uint128::from(ALICE_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let env = mock_env(VOTING_TOKEN.to_string(), &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Bob stakes 50 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(BOB),
        amount: Uint128::from(BOB_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((ALICE_STAKE + BOB_STAKE + CINDY_STAKE + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);
    // Cindy stakes 50 NEB
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(CINDY),
        amount: Uint128::from(CINDY_STAKE),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // Alice votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Abstain,
        amount: Uint128::from(ALICE_STAKE),
    };
    let env = mock_env(ALICE, &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    // Bob votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(BOB_STAKE),
    };
    let env = mock_env(BOB, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    // Cindy votes
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(CINDY_STAKE),
    };
    let env = mock_env(CINDY, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::EndPoll { poll_id: 1 };

    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    // abstain votes make the poll surpass quorum
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "1"),
            log("tallied_weight", "820000000"),
            log("staked_weight", "820000000"),
            log("poll_id", "1"),
            log("rejected_reason", ""),
            log("passed", "true"),
        ]
    );

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let poll_end_height = env.block.height.clone() + DEFAULT_VOTING_PERIOD;
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    // Alice doesn't vote

    // Bob votes
    let msg = HandleMsg::CastVote {
        poll_id: 2,
        vote: VoteOption::Yes,
        amount: Uint128::from(BOB_STAKE),
    };
    let env = mock_env(BOB, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();
    // Cindy votes
    let msg = HandleMsg::CastVote {
        poll_id: 2,
        vote: VoteOption::Yes,
        amount: Uint128::from(CINDY_STAKE),
    };
    let env = mock_env(CINDY, &[]);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::EndPoll { poll_id: 2 };

    let env = mock_env_height(TEST_VOTER, &[], poll_end_height, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    // without abstain votes, quroum is not reached
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0.085365853658536585"),
            log("tallied_weight", "70000000"),
            log("staked_weight", "820000000"),
            log("poll_id", "2"),
            log("rejected_reason", "Quorum not reached"),
            log("passed", "false"),
        ]
    );
}

#[test]
fn test_query_shares() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let voter_0_addr = HumanAddr::from("staker0000");
    let voter_1_addr = HumanAddr::from("staker0001");
    let voter_2_addr = HumanAddr::from("staker0002");

    bank_store(&mut deps.storage)
        .save(
            &voter_0_addr.as_str().as_bytes(),
            &TokenManager {
                share: Uint128(11u128),
                locked_balance: vec![],
                participated_polls: vec![],
                lock_end_week: Some(104u64),
            },
        )
        .unwrap();
    bank_store(&mut deps.storage)
        .save(
            &voter_1_addr.as_str().as_bytes(),
            &TokenManager {
                share: Uint128(22u128),
                locked_balance: vec![],
                participated_polls: vec![],
                lock_end_week: Some(104u64),
            },
        )
        .unwrap();
    bank_store(&mut deps.storage)
        .save(
            &voter_2_addr.as_str().as_bytes(),
            &TokenManager {
                share: Uint128(33u128),
                locked_balance: vec![],
                participated_polls: vec![],
                lock_end_week: Some(104u64),
            },
        )
        .unwrap();

    // query everything Asc
    let res = query(
        &deps,
        QueryMsg::Shares {
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: SharesResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.stakers,
        vec![
            SharesResponseItem {
                staker: HumanAddr::from("staker0000"),
                share: Uint128(11u128),
            },
            SharesResponseItem {
                staker: HumanAddr::from("staker0001"),
                share: Uint128(22u128),
            },
            SharesResponseItem {
                staker: HumanAddr::from("staker0002"),
                share: Uint128(33u128),
            },
        ]
    );

    // query everything Desc
    let res = query(
        &deps,
        QueryMsg::Shares {
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Desc),
        },
    )
    .unwrap();
    let response: SharesResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.stakers,
        vec![
            SharesResponseItem {
                staker: HumanAddr::from("staker0002"),
                share: Uint128(33u128),
            },
            SharesResponseItem {
                staker: HumanAddr::from("staker0001"),
                share: Uint128(22u128),
            },
            SharesResponseItem {
                staker: HumanAddr::from("staker0000"),
                share: Uint128(11u128),
            },
        ]
    );

    // limit 2
    let res = query(
        &deps,
        QueryMsg::Shares {
            start_after: None,
            limit: Some(2u32),
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: SharesResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.stakers,
        vec![
            SharesResponseItem {
                staker: HumanAddr::from("staker0000"),
                share: Uint128(11u128),
            },
            SharesResponseItem {
                staker: HumanAddr::from("staker0001"),
                share: Uint128(22u128),
            },
        ]
    );

    // start after staker0001 and limit 1
    let res = query(
        &deps,
        QueryMsg::Shares {
            start_after: Some(HumanAddr::from("staker0001")),
            limit: Some(1u32),
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let response: SharesResponse = from_binary(&res).unwrap();
    assert_eq!(
        response.stakers,
        vec![SharesResponseItem {
            staker: HumanAddr::from("staker0002"),
            share: Uint128(33u128),
        },]
    );
}

#[test]
fn snapshot_poll() {
    let stake_amount = 1000;

    let mut deps = mock_dependencies(20, &coins(100, VOTING_TOKEN));
    mock_init(&mut deps);

    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let mut creator_env = mock_env(VOTING_TOKEN, &vec![]);
    let handle_res = handle(&mut deps, creator_env.clone(), msg.clone()).unwrap();
    assert_eq!(
        handle_res.log,
        vec![
            log("action", "create_poll"),
            log("creator", TEST_CREATOR),
            log("poll_id", "1"),
            log("end_height", "22345"),
        ]
    );

    //must not be executed
    let snapshot_err = handle(
        &mut deps,
        creator_env.clone(),
        HandleMsg::SnapshotPoll { poll_id: 1 },
    )
    .unwrap_err();
    assert_eq!(
        StdError::generic_err("Cannot snapshot at this height",),
        snapshot_err
    );

    // change time
    creator_env.block.height = 22345 - 10;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let fix_res = handle(
        &mut deps,
        creator_env.clone(),
        HandleMsg::SnapshotPoll { poll_id: 1 },
    )
    .unwrap();

    assert_eq!(
        fix_res.log,
        vec![
            log("action", "snapshot_poll"),
            log("poll_id", "1"),
            log("staked_amount", stake_amount),
        ]
    );

    //must not be executed
    let snapshot_error = handle(
        &mut deps,
        creator_env.clone(),
        HandleMsg::SnapshotPoll { poll_id: 1 },
    )
    .unwrap_err();
    assert_eq!(
        StdError::generic_err("Snapshot has already occurred"),
        snapshot_error
    );
}

#[test]
fn happy_days_cast_vote_with_snapshot() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let env = mock_env_height(VOTING_TOKEN, &vec![], 0, 10000);
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_create_poll_result(
        1,
        DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(11u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(11, DEFAULT_PROPOSAL_DEPOSIT, 11, 1, handle_res, &mut deps);

    //cast_vote without snapshot
    let env = mock_env_height(TEST_VOTER, &coins(11, VOTING_TOKEN), 0, env.block.time);
    let amount = 10u128;

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(amount),
    };

    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_cast_vote_success(TEST_VOTER, amount, 1, VoteOption::Yes, handle_res);

    // balance be double
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(22u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(value.staked_amount, None);
    let end_height = value.end_height;

    //cast another vote
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_2),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    // another voter cast a vote
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(10u128),
    };
    let env = mock_env_height(TEST_VOTER_2, &[], end_height - 9, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_cast_vote_success(TEST_VOTER_2, amount, 1, VoteOption::Yes, handle_res);

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(value.staked_amount, Some(Uint128(22)));

    // snanpshot poll will not go through
    let snap_error = handle(
        &mut deps,
        env.clone(),
        HandleMsg::SnapshotPoll { poll_id: 1 },
    )
    .unwrap_err();
    assert_eq!(
        StdError::generic_err("Snapshot has already occurred"),
        snap_error
    );

    // balance be double
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(33u128 + DEFAULT_PROPOSAL_DEPOSIT),
        )],
    )]);

    // another voter cast a vote but the snapshot is already occurred
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_3),
        amount: Uint128::from(11u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(10u128),
    };
    let env = mock_env_height(TEST_VOTER_3, &[], end_height - 8, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_cast_vote_success(TEST_VOTER_3, amount, 1, VoteOption::Yes, handle_res);

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(value.staked_amount, Some(Uint128(22)));
}

#[test]
fn fails_end_poll_quorum_inflation_without_snapshot_poll() {
    const POLL_START_HEIGHT: u64 = 1000;
    const POLL_ID: u64 = 1;
    let stake_amount = 1000;

    let mut deps = mock_dependencies(20, &coins(1000, VOTING_TOKEN));
    mock_init(&mut deps);

    let mut creator_env = mock_env_height(
        VOTING_TOKEN,
        &coins(2, VOTING_TOKEN),
        POLL_START_HEIGHT,
        10000,
    );

    let exec_msg_bz = to_binary(&Cw20HandleMsg::Burn {
        amount: Uint128(123),
    })
    .unwrap();

    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        None,
        Some(ExecuteMsg {
            contract: HumanAddr::from(VOTING_TOKEN),
            msg: exec_msg_bz.clone(),
        }),
    );

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_create_poll_result(
        1,
        creator_env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env_height(TEST_VOTER, &[], POLL_START_HEIGHT, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "1000"),
            log("voter", TEST_VOTER),
            log("vote_option", "yes"),
        ]
    );

    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD - 10;

    // did not SnapshotPoll

    // staked amount get increased 10 times
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(((10 * stake_amount) + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    //cast another vote
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_2),
        amount: Uint128::from(9 * stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env_height(VOTING_TOKEN, &[], 0, env.block.time);
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    // another voter cast a vote
    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env_height(TEST_VOTER_2, &[], POLL_START_HEIGHT, env.block.time);
    let handle_res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "1000"),
            log("voter", TEST_VOTER_2),
            log("vote_option", "yes"),
        ]
    );

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height += 10;

    // quorum must reach
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "0.2"),
            log("tallied_weight", "2000"),
            log("staked_weight", "10000"),
            log("poll_id", "1"),
            log("rejected_reason", "Quorum not reached"),
            log("passed", "false"),
        ]
    );

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(
        10 * stake_amount,
        value.total_balance_at_end_poll.unwrap().u128()
    );
}

#[test]
fn happy_days_end_poll_with_controlled_quorum() {
    const POLL_START_HEIGHT: u64 = 1000;
    const POLL_ID: u64 = 1;
    let stake_amount = 1000;

    let mut deps = mock_dependencies(20, &coins(1000, VOTING_TOKEN));
    mock_init(&mut deps);

    let mut creator_env = mock_env_height(
        VOTING_TOKEN,
        &coins(2, VOTING_TOKEN),
        POLL_START_HEIGHT,
        10000,
    );

    let exec_msg_bz = to_binary(&Cw20HandleMsg::Burn {
        amount: Uint128(123),
    })
    .unwrap();

    let msg = create_poll_msg(
        "test".to_string(),
        "test".to_string(),
        None,
        Some(ExecuteMsg {
            contract: HumanAddr::from(VOTING_TOKEN),
            msg: exec_msg_bz.clone(),
        }),
    );

    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_create_poll_result(
        1,
        creator_env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(
        stake_amount,
        DEFAULT_PROPOSAL_DEPOSIT,
        stake_amount,
        1,
        handle_res,
        &mut deps,
    );

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env_height(TEST_VOTER, &[], POLL_START_HEIGHT, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "1000"),
            log("voter", TEST_VOTER),
            log("vote_option", "yes"),
        ]
    );

    creator_env.block.height = &creator_env.block.height + DEFAULT_VOTING_PERIOD - 10;
    creator_env.block.time = env.block.time;

    // send SnapshotPoll
    let fix_res = handle(
        &mut deps,
        creator_env.clone(),
        HandleMsg::SnapshotPoll { poll_id: 1 },
    )
    .unwrap();

    assert_eq!(
        fix_res.log,
        vec![
            log("action", "snapshot_poll"),
            log("poll_id", "1"),
            log("staked_amount", stake_amount),
        ]
    );

    // staked amount get increased 10 times
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(((10 * stake_amount) + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    //cast another vote
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_2),
        amount: Uint128::from(9 * stake_amount as u128),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let _handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(9 * stake_amount),
    };
    let env = mock_env_height(TEST_VOTER_2, &[], creator_env.block.height, env.block.time);
    let handle_res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "cast_vote"),
            log("poll_id", POLL_ID),
            log("amount", "9000"),
            log("voter", TEST_VOTER_2),
            log("vote_option", "yes"),
        ]
    );

    creator_env.message.sender = HumanAddr::from(TEST_CREATOR);
    creator_env.block.height += 10;

    // quorum must reach
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let handle_res = handle(&mut deps, creator_env.clone(), msg).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "end_poll"),
            log("quorum", "10"),
            log("tallied_weight", "10000"),
            log("staked_weight", "1000"),
            log("poll_id", "1"),
            log("rejected_reason", ""),
            log("passed", "true"),
        ]
    );
    assert_eq!(
        handle_res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from(VOTING_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from(TEST_CREATOR),
                amount: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
            })
            .unwrap(),
            send: vec![],
        })]
    );

    let res = query(&deps, QueryMsg::Poll { poll_id: 1 }).unwrap();
    let value: PollResponse = from_binary(&res).unwrap();
    assert_eq!(
        stake_amount,
        value.total_balance_at_end_poll.unwrap().u128()
    );

    assert_eq!(value.yes_votes.u128(), 10 * stake_amount);

    // actual staked amount is 10 times bigger than staked amount
    let actual_staked_weight = (load_token_balance(
        &deps,
        &HumanAddr::from(VOTING_TOKEN),
        &HumanAddr::from(MOCK_CONTRACT_ADDR),
    )
    .unwrap()
        - Uint128(DEFAULT_PROPOSAL_DEPOSIT))
    .unwrap();

    assert_eq!(actual_staked_weight.u128(), (10 * stake_amount))
}

#[test]
fn increase_lock_time() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let stake_amount = 1000u128;
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(stake_amount))],
    )]);

    let initial_lock_period = 10u64;
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(initial_lock_period),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(stake_amount, 0, stake_amount, 0, handle_res, &mut deps);

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::from(stake_amount),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );

    let env = mock_env(TEST_VOTER, &[]);
    let msg = HandleMsg::IncreaseLockTime {
        increase_weeks: 95u64,
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone());

    // Should not allow lock time exceeding 104 weeks.
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Lock time exceeds the maximum."),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let increased_lock_period = 20u64;

    let env = mock_env(TEST_VOTER, &[]);
    let msg = HandleMsg::IncreaseLockTime {
        increase_weeks: increased_lock_period,
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        handle_res.log,
        vec![
            log("action", "increase_lock_time"),
            log("sender", "voter1"),
            log("previous_lock_end_week", "2608"),
            log("new_lock_end_week", "2628"),
        ],
    );

    let mut env = mock_env(TEST_VOTER, &[]);

    env.block.time += initial_lock_period * SECONDS_PER_WEEK;

    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(stake_amount)),
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone());

    // Make sure increase_lock_time worked and tokens cannot be withdrawn before lock time expires
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "User is trying to withdraw tokens before expiry.")
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    env.block.time += increased_lock_period * SECONDS_PER_WEEK;
    let msg = HandleMsg::WithdrawVotingTokens {
        amount: Some(Uint128::from(stake_amount)),
    };

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    handle_res.messages.get(0).expect("no message");

    let state: State = state_read(&mut deps.storage).load().unwrap();
    assert_eq!(
        state,
        State {
            contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
            poll_count: 0,
            total_share: Uint128::from(0u128),
            total_deposit: Uint128::zero(),
            pending_voting_rewards: Uint128::zero(),
        }
    );
}

#[test]
fn stake_voting_tokens_multiple_lock_end_weeks() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let stake_amount = 1000u128;
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(stake_amount))],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: None,
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone());

    // Must specify lock_for_weeks when user stakes for the first time
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Must specify lock_for_weeks if no tokens staked.")
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(1000u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone());

    // Must specify lock_for_weeks when user stakes for the first time
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Lock time exceeds the maximum."),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    let lock_period = 10u64;
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(lock_period),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();
    assert_stake_tokens_result(stake_amount, 0, stake_amount, 0, handle_res, &mut deps);

    let new_lock_period = lock_period + 1;
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(new_lock_period),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone());

    // Cannot specify lock for weeks when staking tokens again
    match handle_res {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) =>
            assert_eq!(
                msg,
                "Cannot specify lock_for_weeks if tokens already staked. To change the lock time, use increase_lock_time"
            ),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(2 * stake_amount),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: None,
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env, msg.clone()).unwrap();

    assert_stake_tokens_result(2 * stake_amount, 0, stake_amount, 0, handle_res, &mut deps);
}

#[test]
fn total_voting_power_calculation() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);

    let stake_amount = 1000u128;
    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128(stake_amount))],
    )]);

    let lock_period = 10u64;
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(lock_period),
            })
            .unwrap(),
        ),
    });

    let env = mock_env(VOTING_TOKEN, &[]);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(stake_amount, 0, stake_amount, 0, handle_res, &mut deps);

    let total_voting_power = total_voting_power_read(&mut deps.storage).load().unwrap();
    let current_week = env.block.time / SECONDS_PER_WEEK;

    assert_eq!(
        total_voting_power,
        TotalVotingPower {
            voting_power: [
                Uint128(76),
                Uint128(67),
                Uint128(57),
                Uint128(48),
                Uint128(38),
                Uint128(28),
                Uint128(19),
                Uint128(9),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(96),
                Uint128(86)
            ]
            .to_vec(),
            last_upd: current_week
        }
    );

    let msg = HandleMsg::IncreaseLockTime {
        increase_weeks: 30u64,
    };

    let mut env = mock_env(TEST_VOTER, &[]);

    // Make 5 weeks pass by
    env.block.time += 5 * SECONDS_PER_WEEK;

    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let total_voting_power = total_voting_power_read(&mut deps.storage).load().unwrap();
    let current_week = env.block.time / SECONDS_PER_WEEK;

    assert_eq!(
        total_voting_power,
        TotalVotingPower {
            voting_power: [
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(336),
                Uint128(326),
                Uint128(317),
                Uint128(307),
                Uint128(298),
                Uint128(288),
                Uint128(278),
                Uint128(269),
                Uint128(259),
                Uint128(250),
                Uint128(240),
                Uint128(230),
                Uint128(221),
                Uint128(211),
                Uint128(201),
                Uint128(192),
                Uint128(182),
                Uint128(173),
                Uint128(163),
                Uint128(153),
                Uint128(144),
                Uint128(134),
                Uint128(125),
                Uint128(115),
                Uint128(105),
                Uint128(96),
                Uint128(86),
                Uint128(76),
                Uint128(67),
                Uint128(57),
                Uint128(48),
                Uint128(38),
                Uint128(28),
                Uint128(19),
                Uint128(9),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0)
            ]
            .to_vec(),
            last_upd: current_week
        }
    );

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128(2 * stake_amount),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER_2),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(52u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env_height(VOTING_TOKEN, &[], env.block.height, env.block.time);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_stake_tokens_result(2 * stake_amount, 0, stake_amount, 0, handle_res, &mut deps);

    let total_voting_power = total_voting_power_read(&mut deps.storage).load().unwrap();
    let current_week = env.block.time / SECONDS_PER_WEEK;

    assert_eq!(
        total_voting_power,
        TotalVotingPower {
            voting_power: vec![
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(836),
                Uint128(816),
                Uint128(797),
                Uint128(778),
                Uint128(759),
                Uint128(739),
                Uint128(720),
                Uint128(701),
                Uint128(682),
                Uint128(663),
                Uint128(643),
                Uint128(624),
                Uint128(605),
                Uint128(586),
                Uint128(566),
                Uint128(547),
                Uint128(528),
                Uint128(509),
                Uint128(489),
                Uint128(470),
                Uint128(451),
                Uint128(432),
                Uint128(413),
                Uint128(393),
                Uint128(374),
                Uint128(355),
                Uint128(336),
                Uint128(316),
                Uint128(297),
                Uint128(278),
                Uint128(259),
                Uint128(239),
                Uint128(220),
                Uint128(201),
                Uint128(182),
                Uint128(163),
                Uint128(153),
                Uint128(144),
                Uint128(134),
                Uint128(125),
                Uint128(115),
                Uint128(105),
                Uint128(96),
                Uint128(86),
                Uint128(76),
                Uint128(67),
                Uint128(57),
                Uint128(48),
                Uint128(38),
                Uint128(28),
                Uint128(19),
                Uint128(9),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0),
                Uint128(0)
            ]
            .to_vec(),
            last_upd: current_week
        }
    );
}

#[test]
fn test_unstake_before_claiming_voting_rewards() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        nebula_token: HumanAddr::from(VOTING_TOKEN),
        quorum: Decimal::percent(DEFAULT_QUORUM),
        threshold: Decimal::percent(DEFAULT_THRESHOLD),
        voting_period: DEFAULT_VOTING_PERIOD,
        effective_delay: DEFAULT_EFFECTIVE_DELAY,
        expiration_period: DEFAULT_EXPIRATION_PERIOD,
        proposal_deposit: Uint128(DEFAULT_PROPOSAL_DEPOSIT),
        voter_weight: Decimal::percent(50), // distribute 50% rewards to voters
        snapshot_period: DEFAULT_SNAPSHOT_PERIOD,
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env(VOTING_TOKEN, &coins(2, VOTING_TOKEN));
    let msg = create_poll_msg("test".to_string(), "test".to_string(), None, None);
    let handle_res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_create_poll_result(
        1,
        env.block.height + DEFAULT_VOTING_PERIOD,
        TEST_CREATOR,
        handle_res,
        &mut deps,
    );

    let stake_amount = 100u128;

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_VOTER),
        amount: Uint128::from(stake_amount),
        msg: Some(
            to_binary(&Cw20HookMsg::StakeVotingTokens {
                lock_for_weeks: Some(104u64),
            })
            .unwrap(),
        ),
    });

    let env = mock_env_height(VOTING_TOKEN, &[], env.block.height, env.block.time);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::CastVote {
        poll_id: 1,
        vote: VoteOption::Yes,
        amount: Uint128::from(stake_amount),
    };
    let env = mock_env_height(TEST_VOTER, &[], env.block.height, env.block.time);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + DEFAULT_PROPOSAL_DEPOSIT + 100u128) as u128),
        )],
    )]);

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_COLLECTOR),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&Cw20HookMsg::DepositReward {}).unwrap()),
    });

    let env = mock_env_height(VOTING_TOKEN, &[], env.block.height, env.block.time);
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // END POLL
    let env = mock_env_height(
        TEST_VOTER,
        &[],
        env.block.height + DEFAULT_VOTING_PERIOD,
        env.block.time,
    );
    let msg = HandleMsg::EndPoll { poll_id: 1 };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    deps.querier.with_token_balances(&[(
        &HumanAddr::from(VOTING_TOKEN),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((stake_amount + 100u128) as u128),
        )],
    )]);

    // UNSTAKE VOTING TOKENS
    let msg = HandleMsg::WithdrawVotingTokens { amount: None };
    let mut env = mock_env_height(TEST_VOTER, &[], env.block.height, env.block.time);

    //Make 2 years pass by so lock expires
    env.block.time += 104 * SECONDS_PER_WEEK;
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("recipient", TEST_VOTER),
            log("amount", (stake_amount + 50u128).to_string()), // 100 + 50% of 100
        ]
    );

    let token_manager = bank_read(&mut deps.storage)
        .load(&HumanAddr::from(TEST_VOTER).as_str().as_bytes())
        .unwrap();
    assert_eq!(
        token_manager.locked_balance,
        vec![(
            1u64,
            VoterInfo {
                vote: VoteOption::Yes,
                balance: Uint128::from(stake_amount),
            }
        )]
    );

    // SUCCESS
    let msg = HandleMsg::WithdrawVotingRewards {};
    let env = mock_env_height(TEST_VOTER, &[], 0, 10000);
    let res = handle(&mut deps, env.clone(), msg).unwrap();

    // user can withdraw 50% of total staked (weight = 50% poll share = 100%)
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw_voting_rewards"),
            log("recipient", TEST_VOTER),
            log("amount", 50),
        ]
    );

    // make sure now the state is clean
    let token_manager = bank_read(&mut deps.storage)
        .load(&HumanAddr::from(TEST_VOTER).as_str().as_bytes())
        .unwrap();

    assert_eq!(token_manager.locked_balance, vec![]);

    // expect err
    poll_voter_read(&mut deps.storage, 1u64)
        .load(&HumanAddr::from(TEST_VOTER).as_str().as_bytes())
        .unwrap_err();
}
