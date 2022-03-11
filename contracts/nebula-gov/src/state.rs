use cosmwasm_std::{Addr, Binary, Decimal, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use nebula_protocol::common::OrderBy;
use nebula_protocol::gov::{PollStatus, VoterInfo};

/// config: Config
static KEY_CONFIG: &[u8] = b"config";
/// state: State
static KEY_STATE: &[u8] = b"state";
/// temporary poll id: u64
static KEY_TMP_POLL_ID: &[u8] = b"tmp_poll_id";

/// poll indexer: Bucket<poll_status>; poll_id -> true
static PREFIX_POLL_INDEXER: &[u8] = b"poll_indexer";
/// poll voter: Bucket<poll_id>; address as bytes -> VoterInfo
static PREFIX_POLL_VOTER: &[u8] = b"poll_voter";
/// poll: Poll
static PREFIX_POLL: &[u8] = b"poll";
/// bank: TokenManager
static PREFIX_BANK: &[u8] = b"bank";

/// Maximum number of results when querying.
const MAX_LIMIT: u32 = 30;
/// Default number of results when querying if a limit is not specified.
const DEFAULT_LIMIT: u32 = 10;

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing cluster setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub nebula_token: Addr,
    pub quorum: Decimal,
    pub threshold: Decimal,
    pub voting_period: u64,
    pub effective_delay: u64,
    pub expiration_period: u64, // deprecated, to remove on next state migration
    pub proposal_deposit: Uint128,
    pub voter_weight: Decimal,
    pub snapshot_period: u64,
}

pub fn config_store(storage: &mut dyn Storage) -> Singleton<Config> {
    singleton(storage, KEY_CONFIG)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<Config> {
    singleton_read(storage, KEY_CONFIG)
}

//////////////////////////////////////////////////////////////////////
/// STATE
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing cluster state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// Address of the governance contract
    pub contract_addr: Addr,
    /// Total number of polls
    pub poll_count: u64,
    /// Total staked share. Not equal to the actual total staked due to auto-staked rewards
    pub total_share: Uint128,
    /// Total initial deposits of all polls
    pub total_deposit: Uint128,
    /// Total pending rewards for voters
    pub pending_voting_rewards: Uint128,
}

pub fn state_store(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, KEY_STATE)
}

pub fn state_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, KEY_STATE)
}

//////////////////////////////////////////////////////////////////////
/// TEMP POLL ID
//////////////////////////////////////////////////////////////////////

pub fn store_tmp_poll_id(storage: &mut dyn Storage, tmp_poll_id: u64) -> StdResult<()> {
    singleton(storage, KEY_TMP_POLL_ID).save(&tmp_poll_id)
}

pub fn read_tmp_poll_id(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_TMP_POLL_ID).load()
}

//////////////////////////////////////////////////////////////////////
/// POLL INDEXER (bucket multilevel)
//////////////////////////////////////////////////////////////////////

pub fn poll_indexer_store<'a>(
    storage: &'a mut dyn Storage,
    status: &PollStatus,
) -> Bucket<'a, bool> {
    Bucket::multilevel(
        storage,
        &[PREFIX_POLL_INDEXER, status.to_string().as_bytes()],
    )
}

/// ## Description
/// Returns a list of polls under the provided criterions.
pub fn read_polls<'a>(
    storage: &'a dyn Storage,
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

//////////////////////////////////////////////////////////////////////
/// POLL VOTER (bucket multilevel)
//////////////////////////////////////////////////////////////////////

pub fn poll_voter_store(storage: &mut dyn Storage, poll_id: u64) -> Bucket<VoterInfo> {
    Bucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()])
}

pub fn poll_voter_read(storage: &dyn Storage, poll_id: u64) -> ReadonlyBucket<VoterInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()])
}

/// ## Description
/// Returns a list of poll voters of the specified `poll_id` under the provided criterions.
pub fn read_poll_voters<'a>(
    storage: &'a dyn Storage,
    poll_id: u64,
    start_after: Option<Addr>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<(Addr, VoterInfo)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Asc) => (calc_range_start_addr(start_after), None, OrderBy::Asc),
        _ => (None, calc_range_end_addr(start_after), OrderBy::Desc),
    };

    // Get the `poll_id` poll bucket
    let voters: ReadonlyBucket<'a, VoterInfo> =
        ReadonlyBucket::multilevel(storage, &[PREFIX_POLL_VOTER, &poll_id.to_be_bytes()]);
    voters
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok((
                Addr::unchecked(
                    std::str::from_utf8(&k)
                        .map_err(|_| StdError::invalid_utf8("invalid address"))?
                        .to_string(),
                ),
                v,
            ))
        })
        .collect()
}

//////////////////////////////////////////////////////////////////////
/// POLL (bucket)
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing poll information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Poll {
    /// Poll ID
    pub id: u64,
    /// Poll creator
    pub creator: Addr,
    /// Current poll status
    pub status: PollStatus,
    /// Current YES votes
    pub yes_votes: Uint128,
    /// Current NO votes
    pub no_votes: Uint128,
    /// Current ABSTAIN votes
    pub abstain_votes: Uint128,
    /// End time of the poll voting period
    pub end_time: u64,
    /// Poll title
    pub title: String,
    /// Poll description
    pub description: String,
    /// Poll link
    pub link: Option<String>,
    /// Poll execute data if the poll passes
    pub execute_data: Option<Vec<ExecuteData>>,
    /// Initial deposit amount
    pub deposit_amount: Uint128,
    /// Total balance at the end poll
    pub total_balance_at_end_poll: Option<Uint128>,
    /// Rewards for voters
    pub voters_reward: Uint128,
    /// Total staked amount in the governance contract when snapshotted
    /// -- used for calculating quorum
    pub staked_amount: Option<Uint128>,
}

/// ## Description
/// A custom struct for poll execute data.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExecuteData {
    /// Target contract address to execute a message
    pub contract: Addr,
    /// Message to be executed
    pub msg: Binary,
}

pub fn poll_store(storage: &mut dyn Storage) -> Bucket<Poll> {
    bucket(storage, PREFIX_POLL)
}

pub fn poll_read(storage: &dyn Storage) -> ReadonlyBucket<Poll> {
    bucket_read(storage, PREFIX_POLL)
}

//////////////////////////////////////////////////////////////////////
/// BANK / TOKEN MANAGER (bucket)
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for user's bank / token manager.
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenManager {
    // Total staked share
    pub share: Uint128,
    // A list of poll_id and vote amount
    pub locked_balance: Vec<(u64, VoterInfo)>,
    // A list of the voter's participating poll_id.
    pub participated_polls: Vec<u64>,
}

pub fn bank_store(storage: &mut dyn Storage) -> Bucket<TokenManager> {
    bucket(storage, PREFIX_BANK)
}

pub fn bank_read(storage: &dyn Storage) -> ReadonlyBucket<TokenManager> {
    bucket_read(storage, PREFIX_BANK)
}

/// ## Description
/// Returns a list of stakers under the provided criterions.
pub fn read_bank_stakers<'a>(
    storage: &'a dyn Storage,
    start_after: Option<Addr>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<(Addr, TokenManager)>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Asc) => (calc_range_start_addr(start_after), None, OrderBy::Asc),
        _ => (None, calc_range_end_addr(start_after), OrderBy::Desc),
    };

    // Get the token manager bucket
    let stakers: ReadonlyBucket<'a, TokenManager> = ReadonlyBucket::new(storage, PREFIX_BANK);
    // Query a list of stakers with an address from `start` for `limit` accounts
    stakers
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok((
                Addr::unchecked(
                    std::str::from_utf8(&k)
                        .map_err(|_| StdError::invalid_utf8("invalid address"))?
                        .to_string(),
                ),
                v,
            ))
        })
        .collect()
}

//////////////////////////////////////////////////////////////////////
/// UTILS
//////////////////////////////////////////////////////////////////////

/// ## Description
/// Set the first key after the provided key, by appending a 1 byte.
fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| {
        let mut v = id.to_be_bytes().to_vec();
        v.push(1);
        v
    })
}

/// ## Description
/// Set the first key after the provided key, by appending a 1 byte.
fn calc_range_end(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| id.to_be_bytes().to_vec())
}

/// ## Description
/// Set the first key after the provided key, by appending a 1 byte
fn calc_range_start_addr(start_after: Option<Addr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v = addr.as_bytes().to_vec();
        v.push(1);
        v
    })
}

/// ## Description
/// Set the first key after the provided key, by appending a 1 byte
fn calc_range_end_addr(start_after: Option<Addr>) -> Option<Vec<u8>> {
    start_after.map(|addr| addr.as_bytes().to_vec())
}
