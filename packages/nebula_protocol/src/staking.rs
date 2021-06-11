use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        owner: Option<HumanAddr>,
    },
    RegisterAsset {
        asset_token: HumanAddr,
        staking_token: HumanAddr,
    },

    ////////////////////////
    /// User operations ///
    ////////////////////////
    Unbond {
        asset_token: HumanAddr,
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        // If the asset token is not given, then all rewards are withdrawn
        asset_token: Option<HumanAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Bond { asset_token: HumanAddr },
    DepositReward { rewards: Vec<(HumanAddr, Uint128)> },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PoolInfo {
        asset_token: HumanAddr,
    },
    RewardInfo {
        staker_addr: HumanAddr,
        asset_token: Option<HumanAddr>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoResponse {
    pub asset_token: HumanAddr,
    pub staking_token: HumanAddr,
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponse {
    pub staker_addr: HumanAddr,
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponseItem {
    pub asset_token: HumanAddr,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}
