use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use terraswap::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub nebula_token: String,
    pub terraswap_factory: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        owner: Option<String>,
    },
    RegisterAsset {
        asset_token: String,
        staking_token: String,
    },

    ////////////////////////
    /// User operations ///
    ////////////////////////
    Unbond {
        asset_token: String,
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        // If the asset token is not given, then all rewards are withdrawn
        asset_token: Option<String>,
    },
    /// Provides liquidity and automatically stakes the LP tokens
    AutoStake {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
    },
    /// Hook to stake the minted LP tokens
    AutoStakeHook {
        asset_token: String,
        staking_token: String,
        staker_addr: String,
        prev_staking_token_amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Bond { asset_token: String },
    DepositReward { rewards: Vec<(String, Uint128)> },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PoolInfo {
        asset_token: String,
    },
    RewardInfo {
        staker_addr: String,
        asset_token: Option<String>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub nebula_token: String,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoResponse {
    pub asset_token: String,
    pub staking_token: String,
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponse {
    pub staker_addr: String,
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponseItem {
    pub asset_token: String,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}
