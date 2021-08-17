use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Uint128};

use terraswap::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub protocol_fee_rate: String,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    PostInitialize {
        owner: String,
        terraswap_factory: String,
        nebula_token: String,
        staking_contract: String,
        commission_collector: String,
    },
    UpdateConfig {
        owner: Option<String>,
        token_code_id: Option<u64>,
        cluster_code_id: Option<u64>,
        distribution_schedule: Option<Vec<(u64, u64, Uint128)>>, // [[start_time, end_time, distribution_amount], [], ...]
    },
    UpdateWeight {
        asset_token: String,
        weight: u32,
    },
    CreateCluster {
        /// used to create all necessary contract or register asset
        params: Params,
    },
    PassCommand {
        contract_addr: String,
        msg: Binary,
    },
    DecommissionCluster {
        cluster_contract: String,
        cluster_token: String,
    },

    Distribute {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    ClusterExists { contract_addr: String },
    ClusterList {},
    DistributionInfo {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub nebula_token: String,
    pub staking_contract: String,
    pub commission_collector: String,
    pub protocol_fee_rate: String,
    pub terraswap_factory: String,
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterExistsResponse {
    pub exists: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterListResponse {
    pub contract_infos: Vec<(String, bool)>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    pub weights: Vec<(String, u32)>,
    pub last_distributed: u64,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Params {
    // Name of cluster
    pub name: String,

    // Symbol of cluster
    pub symbol: String,

    // Description of cluster
    pub description: String,

    /// Distribution weight (default is 30, which is 1/10 of NEB distribution weight)
    pub weight: Option<u32>,

    // Corresponding penalty contract to query for mint/redeem
    pub penalty: String,

    /// Pricing oracle address
    pub pricing_oracle: String,

    /// Composition oracle address
    pub composition_oracle: String,

    /// Target assets and weights
    pub target: Vec<Asset>,
}
