use astroport::asset::Asset;
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new cluster contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Cluster's permissioned owner
    pub owner: String,

    /// Factory address
    pub factory: String,

    /// Cluster name (title)
    pub name: String,

    /// Cluster description
    pub description: String,

    /// Cluster token CW20 address
    pub cluster_token: Option<String>,

    /// An address allowed to update asset prices
    pub pricing_oracle: String,

    /// An address allowed to update asset target weights
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
    /// UpdateConfig updates contract setting.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        /// name of the cluster
        name: Option<String>,
        /// description of the cluster
        description: Option<String>,
        /// Cluster token CW20 contract address
        cluster_token: Option<String>,
        /// Pricing oracle address, e.g. contract, bot
        pricing_oracle: Option<String>,
        /// Target oracle address, e.g. contract, bot
        target_oracle: Option<String>,
        /// Penalty contract address for this cluster
        penalty: Option<String>,
        /// Asset target weights
        target: Option<Vec<Asset>>, // recomposition oracle
    },
    /// UpdateTarget changes the asset target weights
    /// -- can also be called by target oracle
    UpdateTarget {
        // new asset target weights to be update
        target: Vec<Asset>,
    },

    /// FACTORY-CALLABLE
    /// Decommission set the cluster to be inactive
    Decommission {},

    /// USER-CALLABLE
    /// RebalanceCreate performs the create operation minting cluster tokens from
    /// provided assets
    RebalanceCreate {
        /// Asset amounts deposited for minting (cluster must be granted allowance or
        /// sent native assets within the MsgExecuteContract)
        asset_amounts: Vec<Asset>,
        /// Minimum cluster tokens to receive
        min_tokens: Option<Uint128>,
    },
    /// RebalanceRedeem performs the redeem operation burning provided cluster tokens
    /// for assets
    RebalanceRedeem {
        /// Maximum cluster tokens to spend
        max_tokens: Uint128,
        /// Proposed set of asset weights to use
        asset_amounts: Option<Vec<Asset>>,
    },
}

/// ## Description
/// This structure describes the possible hook messages for CW20 contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {}

/// ## Description
/// This structure describes the available query messages for the cluster contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    // Target returns the current target weights saved in the contract.
    Target {},
    // ClusterState returns the current cluster state.
    ClusterState {},
    // ClusterInfo returns the name and description of the cluster.
    ClusterInfo {},
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    // The config of the cluster
    pub config: ClusterConfig,
}

/// ## Description
/// A custom struct for each query response that returns the current target weights.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetResponse {
    // The vector of `Asset` in which `amount` is the target weight
    pub target: Vec<Asset>,
}

/// ## Description
/// A custom struct for each query response that returns the current cluster state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterStateResponse {
    // The current total supply of the cluster token
    pub outstanding_balance_tokens: Uint128,
    // Prices of the assets in the cluster
    pub prices: Vec<String>,
    // Current inventory / asset balances
    pub inv: Vec<Uint128>,
    // Penalty contract address
    pub penalty: String,
    // Cluster token address
    pub cluster_token: String,
    // The current asset target weights
    pub target: Vec<Asset>,
    // The address of this cluster contract
    pub cluster_contract_address: String,
    // The cluster active status - not active if decommissioned
    pub active: bool,
}

/// ## Description
/// A custom struct for each query response that returns the cluster info.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterInfoResponse {
    // Cluster name
    pub name: String,
    // Cluster description
    pub description: String,
}

/// ## Description
/// A custom struct for storing cluster setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterConfig {
    // Cluster name
    pub name: String,
    // Cluster description
    pub description: String,
    // Owner of the cluster
    pub owner: Addr,
    // Cluster token contract address
    pub cluster_token: Option<Addr>,
    // Cluster factory contract address
    pub factory: Addr,
    // An address allowed to update asset prices
    pub pricing_oracle: Addr,
    // An address allowed to update target weights
    pub target_oracle: Addr,
    // Penalty contract address of the cluster
    pub penalty: Addr,
    // The cluster active status - not active if decommissioned
    pub active: bool,
}
