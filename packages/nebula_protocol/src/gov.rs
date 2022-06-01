use cosmwasm_std::{Binary, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::common::OrderBy;

/// ## Description
/// This structure stores the basic settings for creating a new governance contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Nebula token address
    pub nebula_token: String,
    /// Poll quorum, threshold for votes in a poll for to be effecive
    pub quorum: Decimal,
    /// Threshold for a poll to pass
    pub threshold: Decimal,
    /// Voting period for each poll
    pub voting_period: u64,
    /// Delay before a poll is executed after the poll passes
    pub effective_delay: u64,
    /// Required amount for an initial deposit
    pub proposal_deposit: Uint128,
    /// Reward ratio for voters, 0-1
    pub voter_weight: Decimal,
    /// Period allowed for a poll snaphost
    pub snapshot_period: u64,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Receive calls a hook message after receiving CW20 asset.
    Receive(Cw20ReceiveMsg),

    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateConfig updates contract setting.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        /// quorum to update
        quorum: Option<Decimal>,
        /// threshold for a poll to pass
        threshold: Option<Decimal>,
        /// voting period
        voting_period: Option<u64>,
        /// delay before a poll is executed
        effective_delay: Option<u64>,
        /// required amount for an initial deposit
        proposal_deposit: Option<Uint128>,
        /// reward ratio for voters, 0-1
        voter_weight: Option<Decimal>,
        /// period allowed for a poll snapshot
        snapshot_period: Option<u64>,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// CastVote adds sender vote to a poll.
    CastVote {
        /// a poll to vote on
        poll_id: u64,
        /// vote option
        vote: VoteOption,
        /// staked amount to vote
        amount: Uint128,
    },
    /// WithdrawVotingTokens withdraws staked token.
    WithdrawVotingTokens {
        /// withdrawn amount
        amount: Option<Uint128>,
    },
    /// WithdrawVotingRewards withdraws voting rewards from a poll.
    WithdrawVotingRewards {
        /// poll id to withdraw reward from
        poll_id: Option<u64>,
    },
    /// StakingVotingRewards stakes voting rewards from a poll.
    StakeVotingRewards {
        /// poll id to retrieve rewards
        poll_id: Option<u64>,
    },
    /// EndPoll finalizes a poll tally.
    EndPoll {
        /// poll id to end
        poll_id: u64,
    },
    /// ExecutePoll runs an execute message after a poll passes
    ExecutePoll {
        /// poll id to execute
        poll_id: u64,
    },
    /// SnapshotPoll saves the current total stake
    SnapshotPoll {
        /// poll id to snapshot
        poll_id: u64,
    },
}

/// ## Description
/// This structure describes the possible hook messages for CW20 contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// StakeVotingTokens a user can stake their Nebula token to receive rewards
    /// or do vote on polls.
    StakeVotingTokens {},
    /// CreatePoll needs to receive deposit from a proposer.
    CreatePoll {
        /// poll title
        title: String,
        /// poll description
        description: String,
        /// poll link
        link: Option<String>,
        /// poll execute message
        execute_msgs: Option<Vec<PollExecuteMsg>>,
    },
    /// DepositReward adds rewards to be distributed among stakers and voters.
    DepositReward {},
}

/// ## Description
/// A custom struct for a poll execute message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PollExecuteMsg {
    /// Contract to execute on
    pub contract: String,
    /// Message to be executed
    pub msg: Binary,
}

/// ## Description
/// This structure describes the available query messages for the governance contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// State returns the current contract state.
    State {},
    /// Staker returns the information of a specific staker.
    Staker {
        /// address to be queried
        address: String,
    },
    /// Poll returns the information of a specific poll.
    Poll {
        /// poll ID to be queried
        poll_id: u64,
    },
    /// Polls returns a list of poll information.
    Polls {
        /// poll status for filtering
        filter: Option<PollStatus>,
        /// starting poll ID of the query result
        start_after: Option<u64>,
        /// maximum number of result
        limit: Option<u32>,
        /// ordering of the result
        order_by: Option<OrderBy>,
    },
    /// Voter returns the information of a specific voter.
    Voter {
        /// poll ID of this vote info
        poll_id: u64,
        /// address of the voter
        address: String,
    },
    /// Voters return a list of voter information
    Voters {
        /// poll ID of the vote info
        poll_id: u64,
        /// starting address of the query result
        start_after: Option<String>,
        /// maximum number of result
        limit: Option<u32>,
        /// ordering of the result
        order_by: Option<OrderBy>,
    },
    /// Shares returns a list of staker shares.
    Shares {
        /// starting address of the query result
        start_after: Option<String>,
        /// maximum number of result
        limit: Option<u32>,
        /// ordering of the result
        order_by: Option<OrderBy>,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Address of the owner
    pub owner: String,
    /// Nebula token contract address
    pub nebula_token: String,
    /// Poll quorum, threshold for votes in a poll for to be effecive
    pub quorum: Decimal,
    /// Threshold for a poll to pass
    pub threshold: Decimal,
    /// Voting period for each poll
    pub voting_period: u64,
    /// Delay before a poll is executed after the poll passes
    pub effective_delay: u64,
    /// Required amount for an initial deposit
    pub proposal_deposit: Uint128,
    /// Reward ratio for voters, 0-1
    pub voter_weight: Decimal,
    /// Period allowed for a poll snaphost
    pub snapshot_period: u64,
}

/// ## Description
/// A custom struct for each query response that returns the state of the governance contract.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// Total number of polls
    pub poll_count: u64,
    /// Total amount of share
    pub total_share: Uint128,
    /// Total amount of current initial deposits
    pub total_deposit: Uint128,
    /// Current pending rewards for voters
    pub pending_voting_rewards: Uint128,
}

/// ## Description
/// A custom struct for each query response that returns an information of the specified poll.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct PollResponse {
    /// Poll ID
    pub id: u64,
    /// Poll creator
    pub creator: String,
    /// Poll status
    pub status: PollStatus,
    /// Poll end height
    pub end_height: u64,
    /// Poll title
    pub title: String,
    /// Poll description
    pub description: String,
    /// Poll link
    pub link: Option<String>,
    /// Poll initial deposit
    pub deposit_amount: Uint128,
    /// Poll execute data
    pub execute_data: Option<Vec<PollExecuteMsg>>,
    /// Amount of YES votes
    pub yes_votes: Uint128,
    /// Amount of NO votes
    pub no_votes: Uint128,
    /// Amount of ABSTAIN votes
    pub abstain_votes: Uint128,
    /// Total staked at end poll
    pub total_balance_at_end_poll: Option<Uint128>,
    /// Rewards for voters
    pub voters_reward: Uint128,
    /// Snapshot staked amount
    pub staked_amount: Option<Uint128>,
}

/// ## Description
/// A custom struct for each query response that returns a list of multiple poll information.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct PollsResponse {
    /// A list of poll information
    pub polls: Vec<PollResponse>,
}

/// ## Description
/// A custom struct for each query response that returns an information of the specified staker.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct StakerResponse {
    /// Current staked amount
    pub balance: Uint128,
    /// Current share
    pub share: Uint128,
    /// A list of vote info, containing voting amount
    pub locked_balance: Vec<(u64, VoterInfo)>,
    /// Polls that the staker can withdraw rewards from
    pub withdrawable_polls: Vec<(u64, Uint128)>,
    /// Pending rewards of the staker
    pub pending_voting_rewards: Uint128,
}

/// ## Description
/// A custom struct for a staker address and their current share.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct SharesResponseItem {
    /// Address of a staker
    pub staker: String,
    /// Corresponding share
    pub share: Uint128,
}

/// ## Description
/// A custom struct for each query response that returns a list of staker shares
/// in a custom [`SharesResponseItem`] structure.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct SharesResponse {
    /// A list of stakers and their shares
    pub stakers: Vec<SharesResponseItem>,
}

/// ## Description
/// A custom struct for each query response that returns a vote information on a poll.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct VotersResponseItem {
    /// Address of a voter
    pub voter: String,
    /// Vote option
    pub vote: VoteOption,
    /// Vote amount
    pub balance: Uint128,
}

/// ## Description
/// A custom struct for each query response that returns a list of voter information on a poll
/// in a custom [`VotersResponseItem`] structure.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct VotersResponse {
    /// A list of voters on a poll
    pub voters: Vec<VotersResponseItem>,
}

/// ## Description
/// A struct used for migrating contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// ## Description
/// A custom struct for a vote information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VoterInfo {
    /// Vote option
    pub vote: VoteOption,
    /// Vote amount
    pub balance: Uint128,
}

/// ## Description
/// A custom enum for vote status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PollStatus {
    InProgress,
    Passed,
    Rejected,
    Executed,
    Expired,
    Failed,
}

impl fmt::Display for PollStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// ## Description
/// A custom enum for vote option.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoteOption {
    Yes,
    No,
    Abstain,
}

impl fmt::Display for VoteOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VoteOption::Yes => write!(f, "yes"),
            VoteOption::No => write!(f, "no"),
            VoteOption::Abstain => write!(f, "abstain"),
        }
    }
}
