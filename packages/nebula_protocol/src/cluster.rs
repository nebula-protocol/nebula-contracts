use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Cluster's permissioned owner
    pub owner: String,

    /// Factory address
    pub factory: String,

    /// Cluster name (title)
    pub name: String,

    /// Cluster description (title)
    pub description: String,

    /// Cluster token CW20 address
    pub cluster_token: Option<String>,

    /// Pricing oracle address
    pub pricing_oracle: String,

    /// Target target oracle address
    pub target_oracle: String,

    /// Asset addresses and target weights
    pub target: Vec<Asset>,

    /// Penalty function address
    pub penalty: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// OWNER-CALLABLE
    UpdateConfig {
        owner: Option<String>,
        name: Option<String>,
        description: Option<String>,
        cluster_token: Option<String>,
        pricing_oracle: Option<String>,
        target_oracle: Option<String>,
        penalty: Option<String>,
        target: Option<Vec<Asset>>, // recomp oracle
    },
    /// Called by target oracle
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
        cluster_contract_address: String,
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
    pub penalty: String,
    pub cluster_token: String,
    pub target: Vec<Asset>,
    pub cluster_contract_address: String,
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
    pub owner: String,
    pub cluster_token: Option<String>,
    pub factory: String,
    pub pricing_oracle: String,
    pub target_oracle: String,
    pub penalty: String,
    pub active: bool,
}
