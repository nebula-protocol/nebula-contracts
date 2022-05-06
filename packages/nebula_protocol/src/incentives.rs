use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::pair::PoolResponse as AstroportPoolResponse;
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

/// ## Description
/// This structure stores the basic settings for creating a new incentives contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the contract, Gov contract
    pub owner: String,
    /// Proxy contract
    pub proxy: String,
    /// Custody contract
    pub custody: String,
    /// Nebula token contract
    pub nebula_token: String,
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

    /// UpdateConfig updates contract owner.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: String,
    },
    /// NewPenaltyPeriod increases the penalty period by one.
    NewPenaltyPeriod {},

    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// RecordAstroportImpact records arbitrage contribution for the reward distribution.
    RecordAstroportImpact {
        /// an address performing the arbitrage
        arbitrageur: Addr,
        /// Astroport pair contract
        astroport_pair: Addr,
        /// cluster contract
        cluster_contract: Addr,
        /// Astroport pair pool state before arbitrage
        pool_before: AstroportPoolResponse,
    },
    /// RecordRebalancerRewards records rebalance contribution for the reward distribution.
    RecordRebalancerRewards {
        /// an address performing the rebalance
        rebalancer: Addr,
        /// cluster contract
        cluster_contract: Addr,
        /// cluster inventory before rebalance
        original_inventory: Vec<Uint128>,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// Withdraw withdraws all rewards for the sender.
    Withdraw {},
}

/// ## Description
/// This structure describes the available query messages for the incentives contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// PenaltyPeriod returns the current penalty period.
    PenaltyPeriod {},
    /// PoolInfo returns the pool info corresponding to the pool type, cluster address,
    /// and penalty period.
    PoolInfo {
        /// pool reward type
        pool_type: u16,
        /// cluster contract
        cluster_address: String,
        /// penalty period
        n: Option<u64>,
    },
    /// CurrentContributorInfo returns the contributor info corresponding to the pool type,
    /// cluter address, and cluster address.
    CurrentContributorInfo {
        /// pool reward type
        pool_type: u16,
        /// contributor address
        contributor_address: String,
        /// cluster contract
        cluster_address: String,
    },
    /// ContributorPendingRewards returns the all pending rewards of a specific contributor.
    ContributorPendingRewards {
        /// contributor address
        contributor_address: String,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the contract, Gov contract
    pub owner: String,
    /// Proxy contract
    pub proxy: String,
    /// Incentives custody contract
    pub custody: String,
    /// Nebula token contract
    pub nebula_token: String,
}

/// ## Description
/// A custom struct for each query that returns the current penalty period.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyPeriodResponse {
    /// The current penalty period
    pub n: u64,
}

/// ## Description
/// A custom struct for each query that returns the information of a reward pool containing
/// the total contribution to this pool and the total rewards to be distributed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IncentivesPoolInfoResponse {
    /// Total contribution value to this reward pool
    pub value_total: Uint128,
    /// Total rewards to be distributed
    pub reward_total: Uint128,
}

/// ## Description
/// A custom struct for each query that returns the information of a specific contribution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentContributorInfoResponse {
    /// Latest penalty period of this contribution
    pub n: u64,
    /// Contribution to in this penalty period
    pub value_contributed: Uint128,
}

/// ## Description
/// A custom struct for each query that returns the pending rewards for a contributor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContributorPendingRewardsResponse {
    /// Pending rewards for the contributor
    pub pending_rewards: Uint128,
}

/// ## Description
/// This structure describes the possible hook messages for CW20 contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// DepositReward adds rewards to be distributed among stakers and voters.
    DepositReward {
        /// a list of rewards to be deposited
        /// (pool_type, cluster address, amount)
        rewards: Vec<(u16, String, Uint128)>,
    },
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// ## Description
/// External execute messages to execute other contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtExecuteMsg {
    /// RequestNeb gets Nebula tokens from the incentives custody contract.
    RequestNeb {
        /// amount of Nebula tokens requested
        amount: Uint128,
    },
}

/// ## Description
/// A custom struct specifying reward pool type constants.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolType;

impl PoolType {
    /// Rebalance reward pool
    pub const REBALANCE: u16 = 0;
    /// Arbitrage reward pool
    pub const ARBITRAGE: u16 = 1;

    /// All possible reward pools
    pub const ALL_TYPES: [&'static u16; 2] = [&0u16, &1u16];
}
