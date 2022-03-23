use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::Asset;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

/// ## Description
/// This structure stores the basic settings for creating a new LP staking contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// Nebula token contract
    pub nebula_token: String,
    /// Astroport factory contract
    pub astroport_factory: String,
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
    },
    /// RegisterAsset registers a new LP staking pool.
    RegisterAsset {
        /// cluster token contract
        asset_token: String,
        /// LP token contract
        staking_token: String,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// Unbond unstakes the specific LP token.
    Unbond {
        /// cluster token contract
        asset_token: String,
        /// amount to unbond
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        /// if the asset token is not given, then all rewards are withdrawn
        asset_token: Option<String>,
    },
    /// Provides liquidity and automatically stakes the LP tokens
    AutoStake {
        /// assets to provide pool liquidity
        assets: [Asset; 2],
        /// the maximum percent of price movement when providing liquidity
        slippage_tolerance: Option<Decimal>,
    },

    /////////////////////
    /// INTERNAL
    /////////////////////

    /// Hook to stake the minted LP tokens
    AutoStakeHook {
        /// cluster token contract
        asset_token: Addr,
        /// cluster LP token contract
        staking_token: Addr,
        /// staker address
        staker_addr: Addr,
        /// the LP token balance of this contract before providing pool liquidity
        prev_staking_token_amount: Uint128,
    },
}

/// ## Description
/// This structure describes the possible hook messages for CW20 contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Bond stakes the specific LP token.
    Bond {
        /// cluster token contract
        asset_token: String,
    },
    /// DepositRewards adds rewards to LP staking pools.
    DepositReward {
        /// list of deposited rewards
        /// -- (cluster contract, reward amount)
        rewards: Vec<(String, Uint128)>,
    },
}

/// ## Description
/// This structure describes the available query messages for the LP staking contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// PoolInfo returns information of a LP staking pool.
    PoolInfo {
        /// cluster token contract
        asset_token: String,
    },
    /// RewardInfo reward information of a LP staker from a specific LP staking pool.
    RewardInfo {
        /// staker address
        staker_addr: String,
        /// cluster token contract. If not specified, return all pool rewards
        asset_token: Option<String>,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the contract
    pub owner: String,
    /// Astroport factory contract address
    pub astroport_factory: String,
    /// Nebula token contract
    pub nebula_token: String,
}

/// ## Description
/// A custom struct for each query response that returns LP staking pool information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoResponse {
    /// Cluster token contract
    pub asset_token: String,
    /// Cluster LP token contract
    pub staking_token: String,
    /// Total bond in the pool
    pub total_bond_amount: Uint128,
    /// Index for reward distribution
    pub reward_index: Decimal,
    /// Pool pending rewards
    pub pending_reward: Uint128,
}

/// ## Description
/// A custom struct for each query response that returns a list of LP staking infos and rewards
/// in the custom [`RewardInfoResponseItem`] structure.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponse {
    /// Staker address
    pub staker_addr: String,
    /// LP staking rewards of the staker
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

/// ## Description
/// A custom struct for storing LP staking infos and rewards in a pool.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponseItem {
    /// Cluster token contract
    pub asset_token: String,
    /// Bond amount of a staker
    pub bond_amount: Uint128,
    /// Pending reward from this pool for a staker
    pub pending_reward: Uint128,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
