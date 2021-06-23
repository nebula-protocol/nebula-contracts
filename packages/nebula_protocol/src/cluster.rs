use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};
use terraswap::hook::InitHook;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Cluster name (title)
    pub name: String,

    /// Cluster's permissioned owner
    pub owner: HumanAddr,

    /// Cluster token CW20 address
    pub cluster_token: Option<HumanAddr>,

    /// Asset addresses
    pub assets: Vec<AssetInfo>,

    /// Factory address
    pub factory: HumanAddr,

    /// Pricing oracle address
    pub pricing_oracle: HumanAddr,

    /// Target composition oracle address
    pub composition_oracle: HumanAddr,

    /// Penalty function address
    pub penalty: HumanAddr,

    /// Target weight vector (not normalized)
    pub target: Vec<u32>,

    pub init_hook: Option<InitHook>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Called to set cluster token after initialization
    _SetClusterToken {
        cluster_token: HumanAddr,
    },

    /// Can be called by the owner to reset the composition oracle
    ResetCompositionOracle {
        composition_oracle: HumanAddr,
    },

    /// Can be called by the owner to reset the cluster owner
    _ResetOwner {
        owner: HumanAddr,
    },

    /// Can be called by the owner to reset the cluster weight target
    ResetTarget {
        assets: Vec<AssetInfo>,
        target: Vec<u32>,
    },

    ResetPenalty {
        penalty: HumanAddr,
    },

    /// Mints new assets
    Mint {
        /// Asset amounts deposited for minting (must be staged)
        asset_amounts: Vec<Asset>,
        /// Minimum tokens to receive
        min_tokens: Option<Uint128>,
    },
    /// Burns assets
    Burn {
        /// optional proposed set of weights to use
        max_tokens: Uint128,
        asset_amounts: Option<Vec<Asset>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Target {},
    ClusterState { cluster_contract_address: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: ClusterConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetResponse {
    pub assets: Vec<AssetInfo>,
    pub target: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterStateResponse {
    pub outstanding_balance_tokens: Uint128,
    pub prices: Vec<String>,
    pub inv: Vec<Uint128>,
    pub assets: Vec<AssetInfo>,
    pub penalty: HumanAddr,
    pub cluster_token: HumanAddr,
    pub target: Vec<u32>,
    pub cluster_contract_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterConfig {
    pub name: String,
    pub owner: HumanAddr,
    pub cluster_token: Option<HumanAddr>,
    pub factory: HumanAddr,
    pub pricing_oracle: HumanAddr,
    pub composition_oracle: HumanAddr,
    pub penalty: HumanAddr,
}
