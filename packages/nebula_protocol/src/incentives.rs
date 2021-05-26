use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use terraswap::asset::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub factory: HumanAddr,
    pub terraswap_factory: HumanAddr,
    pub nebula_token: HumanAddr,
    pub base_denom: String,
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    _ResetOwner {
        owner: HumanAddr,
    },
    Receive(Cw20ReceiveMsg),
    Withdraw {},
    NewPenaltyPeriod {},

    SendAll {
        asset_infos: Vec<AssetInfo>,
        send_to: HumanAddr,
    },

    SwapAll {
        terraswap_pair: HumanAddr,
        basket_token: HumanAddr,
        to_ust: bool,
    },

    RecordTerraswapImpact {
        arbitrager: HumanAddr,
        terraswap_pair: HumanAddr,
        basket_contract: HumanAddr,
        pool_before: PoolResponse
    },

    ArbClusterMint {
        basket_contract: HumanAddr,
        assets: Vec<Asset>
    },

    ArbClusterRedeem {
        basket_contract: HumanAddr,
        asset: Asset
    },

    Mint {
        basket_contract: HumanAddr,
        asset_amounts: Vec<Asset>,
        min_tokens: Option<Uint128>,
    },

    Redeem {
        basket_contract: HumanAddr,
        max_tokens: Uint128,
        asset_amounts: Option<Vec<Asset>>,
    },

    _InternalRewardedMint {
        rebalancer: HumanAddr,
        basket_contract: HumanAddr,
        asset_amounts: Vec<Asset>,
        min_tokens: Option<Uint128>,
    },

    _InternalRewardedRedeem {
        rebalancer: HumanAddr,
        basket_contract: HumanAddr,
        basket_token: HumanAddr,
        max_tokens: Option<Uint128>,
        asset_amounts: Option<Vec<Asset>>,
    },

    _RecordRebalancerRewards {
        rebalancer: HumanAddr,
        basket_contract: HumanAddr,
        original_imbalance: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub factory: HumanAddr,
    pub terraswap_factory: HumanAddr,
    pub nebula_token: HumanAddr,
    pub base_denom: String,
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit rewards to be distributed among stakers and voters
    DepositReward { rewards: Vec<(u16, HumanAddr, Uint128)> },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtQueryMsg {
    Pool {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

pub struct PoolType;

impl PoolType {
    pub const REBALANCER: u16 = 0;
    pub const ARBITRAGER: u16 = 1;

    pub const ALL_TYPES: [&'static u16; 2] = [&0u16, &1u16];
}