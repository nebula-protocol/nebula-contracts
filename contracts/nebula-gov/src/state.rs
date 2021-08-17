use cosmwasm_std::{Binary, Decimal, HumanAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cluster_math::FPDecimal;

use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{PollExecuteMsg, PollStatus, VoterInfo};

static KEY_CONFIG: &[u8] = b"config";
static KEY_STATE: &[u8] = b"state";
static KEY_TOTAL_VOTING_POWER: &[u8] = b"total_voting_power";

static PREFIX_POLL_INDEXER: &[u8] = b"poll_indexer";
static PREFIX_POLL_VOTER: &[u8] = b"poll_voter";
static PREFIX_POLL: &[u8] = b"poll";
static PREFIX_BANK: &[u8] = b"bank";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
    pub quorum: Decimal,
    pub threshold: Decimal,
    pub voting_period: u64,
    pub effective_delay: u64,
    pub expiration_period: u64,
    pub proposal_deposit: Uint128,
    pub voter_weight: Decimal,
    pub snapshot_period: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub contract_addr: HumanAddr,
    pub poll_count: u64,
    pub total_share: Uint128,
    pub total_deposit: Uint128,
    pub pending_voting_rewards: Uint128,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenManager {
    pub share: Uint128,                        // total staked balance
    pub locked_balance: Vec<(u64, VoterInfo)>, // maps poll_id to weight voted
    pub participated_polls: Vec<u64>,          // poll_id
    pub lock_end_week: Option<u64>,            // time when lock on staked tokens expires
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Poll {
    pub id: u64,
    pub creator: HumanAddr,
    pub status: PollStatus,
    pub yes_votes: Uint128,
    pub no_votes: Uint128,
    pub abstain_votes: Uint128,
    pub end_height: u64,
    pub title: String,
    pub description: String,
    pub link: Option<String>,
    pub execute_data: Option<PollExecuteMsg>,
    pub deposit_amount: Uint128,
    /// Total balance at the end poll
    pub total_balance_at_end_poll: Option<Uint128>,
    pub voters_reward: Uint128,
    pub staked_amount: Option<Uint128>,

    pub max_voting_power: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExecuteData {
    pub contract: HumanAddr,
    pub msg: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalVotingPower {
    pub voting_power: Vec<FPDecimal>,
    pub last_upd: u64,
}

pub fn config_store(storage: &mut dyn Storage) -> Singleton<Config> {
    singleton(storage, KEY_CONFIG)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<Config> {
    singleton_read(storage, KEY_CONFIG)
}

pub fn state_store(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, KEY_STATE)
}

pub fn state_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, KEY_STATE)
}

pub fn poll_store(storage: &mut dyn Storage) -> Bucket<Poll> {
    bucket(PREFIX_POLL, storage)
}

pub fn poll_read(storage: &dyn Storage) -> ReadonlyBucket<Poll> {
    bucket_read(PREFIX_POLL, storage)
}

pub fn total_voting_power_store(storage: &mut dyn Storage) -> Singleton<TotalVotingPower> {
    singleton(storage, KEY_TOTAL_VOTING_POWER)
}

pub fn total_voting_power_read(storage: &dyn Storage) -> ReadonlySingleton<TotalVotingPower> {
    singleton_read(storage, KEY_TOTAL_VOTING_POWER)
}

pub fn poll_indexer_store<'a>(
    storage: &'a mut Storage,
    status: &PollStatus,
) -> Bucket<'a, bool> {
    Bucket::multilevel(
        storage,
        &[PREFIX_POLL_INDEXER, status.to_string().as_bytes()],
    )
}

pub fn poll_voter_store(storage: &mut dyn Storage, poll_id: u64) -> Bucket<VoterInfo> {
    Bucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()])
}

pub fn poll_voter_read(storage: &dyn Storage, poll_id: u64) -> ReadonlyBucket<VoterInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()])
}

pub fn read_poll_voters<'a>(
    storage: &'a Storage,
    poll_id: u64,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<(HumanAddr, VoterInfo)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Asc) => (calc_range_start_addr(start_after), None, OrderBy::Asc),
        _ => (None, calc_range_end_addr(start_after), OrderBy::Desc),
    };

    let voters: ReadonlyBucket<'a, VoterInfo> =
        ReadonlyBucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()]);
    voters
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok((
                HumanAddr::from(unsafe { std::str::from_utf8_unchecked(&k) }),
                v,
            ))
        })
        .collect()
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_polls<'a>(
    storage: &'a Storage,
    filter: Option<PollStatus>,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
    remove_hard_cap: Option<bool>,
) -> StdResult<Vec<Poll>> {
    let mut limit: usize = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    if let Some(remove_hard_cap) = remove_hard_cap {
        if remove_hard_cap {
            limit = usize::MAX;
        }
    }
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Asc) => (calc_range_start(start_after), None, OrderBy::Asc),
        _ => (None, calc_range_end(start_after), OrderBy::Desc),
    };

    if let Some(status) = filter {
        let poll_indexer: ReadonlyBucket<'a, bool> = ReadonlyBucket::multilevel(
            storage,
            &[PREFIX_POLL_INDEXER, status.to_string().as_bytes()],
        );
        poll_indexer
            .range(start.as_deref(), end.as_deref(), order_by.into())
            .take(limit)
            .map(|item| {
                let (k, _) = item?;
                poll_read(storage).load(&k)
            })
            .collect()
    } else {
        let polls: ReadonlyBucket<'a, Poll> = ReadonlyBucket::new(storage, PREFIX_POLL);

        polls
            .range(start.as_deref(), end.as_deref(), order_by.into())
            .take(limit)
            .map(|item| {
                let (_, v) = item?;
                Ok(v)
            })
            .collect()
    }
}

pub fn bank_store(storage: &mut dyn Storage) -> Bucket<TokenManager> {
    bucket(PREFIX_BANK, storage)
}

pub fn bank_read(storage: &dyn Storage) -> ReadonlyBucket<TokenManager> {
    bucket_read(PREFIX_BANK, storage)
}

pub fn read_bank_stakers<'a>(
    storage: &'a Storage,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<(HumanAddr, TokenManager)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Asc) => (calc_range_start_addr(start_after), None, OrderBy::Asc),
        _ => (None, calc_range_end_addr(start_after), OrderBy::Desc),
    };

    let stakers: ReadonlyBucket<'a, TokenManager> = ReadonlyBucket::new(storage, PREFIX_BANK);
    stakers
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok((
                HumanAddr::from(unsafe { std::str::from_utf8_unchecked(&k) }),
                v,
            ))
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| {
        let mut v = id.to_be_bytes().to_vec();
        v.push(1);
        v
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_end(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| id.to_be_bytes().to_vec())
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start_addr(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v = addr.as_str().as_bytes().to_vec();
        v.push(1);
        v
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_end_addr(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    start_after.map(|addr| addr.as_str().as_bytes().to_vec())
}
