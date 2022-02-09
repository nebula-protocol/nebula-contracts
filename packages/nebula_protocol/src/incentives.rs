use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::{Asset, AssetInfo};
use astroport::pair::PoolResponse as AstroportPoolResponse;
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub factory: String,
    pub custody: String,
    pub astroport_factory: String,
    pub nebula_token: String,
    pub base_denom: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// OWNER-CALLABLE
    UpdateOwner {
        owner: String,
    },
    Receive(Cw20ReceiveMsg),
    Withdraw {},
    NewPenaltyPeriod {},

    /// INTERNAL
    _SendAll {
        asset_infos: Vec<AssetInfo>,
        send_to: Addr,
    },

    _SwapAll {
        astroport_pair: Addr,
        cluster_token: Addr,
        min_return: Option<Uint128>,
        to_ust: bool,
    },

    _RecordAstroportImpact {
        arbitrageur: Addr,
        astroport_pair: Addr,
        cluster_contract: Addr,
        pool_before: AstroportPoolResponse,
    },
    _InternalRewardedCreate {
        rebalancer: Addr,
        cluster_contract: Addr,
        asset_amounts: Vec<Asset>,
        min_tokens: Option<Uint128>,
    },

    _InternalRewardedRedeem {
        rebalancer: Addr,
        cluster_contract: Addr,
        cluster_token: Addr,
        max_tokens: Option<Uint128>,
        asset_amounts: Option<Vec<Asset>>,
    },

    _RecordRebalancerRewards {
        rebalancer: Addr,
        cluster_contract: Addr,
        original_imbalance: Uint128,
    },

    /// USER-CALLABLE
    ArbClusterCreate {
        cluster_contract: String,
        assets: Vec<Asset>,
        min_ust: Option<Uint128>,
    },

    ArbClusterRedeem {
        cluster_contract: String,
        asset: Asset,
        min_cluster: Option<Uint128>,
    },

    IncentivesCreate {
        cluster_contract: String,
        asset_amounts: Vec<Asset>,
        min_tokens: Option<Uint128>,
    },

    IncentivesRedeem {
        cluster_contract: String,
        max_tokens: Uint128,
        asset_amounts: Option<Vec<Asset>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PenaltyPeriod {},
    PoolInfo {
        pool_type: u16,
        cluster_address: String,
        n: Option<u64>,
    },
    CurrentContributorInfo {
        pool_type: u16,
        contributor_address: String,
        cluster_address: String,
    },
    ContributorPendingRewards {
        contributor_address: String,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub factory: String,
    pub astroport_factory: String,
    pub nebula_token: String,
    pub base_denom: String,
    pub owner: String,
    pub custody: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyPeriodResponse {
    pub n: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IncentivesPoolInfoResponse {
    pub value_total: Uint128,
    pub reward_total: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentContributorInfoResponse {
    pub n: u64,
    pub value_contributed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContributorPendingRewardsResponse {
    pub pending_rewards: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit rewards to be distributed among stakers and voters
    DepositReward {
        rewards: Vec<(u16, String, Uint128)>,
    },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtExecuteMsg {
    RequestNeb { amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolType;

impl PoolType {
    pub const REBALANCE: u16 = 0;
    pub const ARBITRAGE: u16 = 1;

    pub const ALL_TYPES: [&'static u16; 2] = [&0u16, &1u16];
}
