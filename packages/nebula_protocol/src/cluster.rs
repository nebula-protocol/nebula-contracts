use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Cluster's permissioned owner
    pub owner: HumanAddr,

    /// Factory address
    pub factory: HumanAddr,

    /// Cluster name (title)
    pub name: String,

    /// Cluster description (title)
    pub description: String,

    /// Cluster token CW20 address
    pub cluster_token: Option<HumanAddr>,

    /// Pricing oracle address
    pub pricing_oracle: HumanAddr,

    /// Target composition oracle address
    pub composition_oracle: HumanAddr,

    /// Asset addresses and target weights
    pub target: Vec<Asset>,

    /// Penalty function address
    pub penalty: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// OWNER-CALLABLE
    UpdateConfig {
        owner: Option<HumanAddr>,
        name: Option<String>,
        description: Option<String>,
        cluster_token: Option<HumanAddr>,
        pricing_oracle: Option<HumanAddr>,
        composition_oracle: Option<HumanAddr>,
        penalty: Option<HumanAddr>,
        target: Option<Vec<Asset>>, // recomp oracle
    },
    /// Called by recomposition oracle
    UpdateTarget { target: Vec<Asset> },

    /// Called by factory only and sets active to false
    Decommission {},

    /// USER-CALLABLE
    Mint {
        /// Asset amounts deposited for minting (cluster must be granted allowance or
        /// sent native assets within the MsgExecuteContract)
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
    ClusterState {
        /// we need to pass in address of cluster we want to fetch state on
        /// query in CosmWasm v0.10 does not have env.contract_address
        /// NOTE: remove for col-5
        cluster_contract_address: HumanAddr,
    },
    ClusterInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: ClusterConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetResponse {
    pub target: Vec<Asset>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterStateResponse {
    pub outstanding_balance_tokens: Uint128,
    pub prices: Vec<String>,
    pub inv: Vec<Uint128>,
    pub penalty: HumanAddr,
    pub cluster_token: HumanAddr,
    pub target: Vec<Asset>,
    pub cluster_contract_address: HumanAddr,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterInfoResponse {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterConfig {
    pub name: String,
    pub description: String,
    pub owner: HumanAddr,
    pub cluster_token: Option<HumanAddr>,
    pub factory: HumanAddr,
    pub pricing_oracle: HumanAddr,
    pub composition_oracle: HumanAddr,
    pub penalty: HumanAddr,
    pub active: bool,
}
