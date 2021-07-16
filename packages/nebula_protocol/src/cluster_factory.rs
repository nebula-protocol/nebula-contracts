use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, HumanAddr, Uint128};

use terraswap::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub protocol_fee_rate: String,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    PostInitialize {
        owner: HumanAddr,
        terraswap_factory: HumanAddr,
        nebula_token: HumanAddr,
        staking_contract: HumanAddr,
        oracle_contract: HumanAddr,
        commission_collector: HumanAddr,
    },
    UpdateConfig {
        owner: Option<HumanAddr>,
        token_code_id: Option<u64>,
        cluster_code_id: Option<u64>,
        distribution_schedule: Option<Vec<(u64, u64, Uint128)>>, // [[start_time, end_time, distribution_amount], [], ...]
    },
    UpdateWeight {
        asset_token: HumanAddr,
        weight: u32,
    },
    CreateCluster {
        /// used to create all necessary contract or register asset
        params: Params,
    },
    /// Internal use
    TokenCreationHook {},
    /// Internal use
    TerraswapCreationHook {
        asset_token: HumanAddr,
    },
    /// Internal use - Set Cluster Token
    SetClusterTokenHook {
        cluster: HumanAddr,
    },
    PassCommand {
        contract_addr: HumanAddr,
        msg: Binary,
    },

    Distribute {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    ClusterExists { contract_addr: HumanAddr },
    ClusterList {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
    pub staking_contract: HumanAddr,
    pub commission_collector: HumanAddr,
    pub protocol_fee_rate: String,
    pub oracle_contract: HumanAddr,
    pub terraswap_factory: HumanAddr,
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
    pub contract_addrs: Vec<HumanAddr>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    pub weights: Vec<(HumanAddr, u32)>,
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

    /// Distribution weight (default is 30, which is 1/10 of NEB distribution weight)
    pub weight: Option<u32>,

    // Corresponding penalty contract to query for mint/redeem
    pub penalty: HumanAddr,

    /// Pricing oracle address
    pub pricing_oracle: HumanAddr,

    /// Composition oracle address
    pub composition_oracle: HumanAddr,

    pub assets: Vec<AssetInfo>,

    pub target: Vec<u32>,
}
