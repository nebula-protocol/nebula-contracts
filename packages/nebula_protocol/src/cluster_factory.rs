use astroport::asset::Asset;
use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new factory contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Code ID of the uploaded CW20 contract code
    pub token_code_id: u64,

    /// Code ID of the uploaded cluster contract code
    pub cluster_code_id: u64,

    /// Base denom used in all clusters
    pub base_denom: String,

    /// Fee rate of create / redeem processes
    pub protocol_fee_rate: String,

    /// Distribution schedule of Nebula token rewards
    /// [[start_time, end_time, distribution_amount], [], ...]
    pub distribution_schedule: Vec<(u64, u64, Uint128)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// After Init Only
    /////////////////////

    /// PostInitialize updates the rest of factory setting after initializing other Nebula contracts.
    PostInitialize {
        /// address to claim the contract ownership
        owner: String,
        /// Astroport factory contract address
        astroport_factory: String,
        /// Nebula token contract address
        nebula_token: String,
        /// LP staking contract address
        staking_contract: String,
        /// collector contract address
        commission_collector: String,
    },

    /////////////////////
    /// Owner Callable
    /////////////////////

    /// UpdateConfig updates contract setting.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        /// code id of the uploaded CW20 contract code
        token_code_id: Option<u64>,
        /// code id of the uploaded cluster contract code
        cluster_code_id: Option<u64>,
        /// [[start_time, end_time, distribution_amount], [], ...]
        distribution_schedule: Option<Vec<(u64, u64, Uint128)>>,
    },
    /// UpdateWeight changes reward distribution weight of
    /// the cluster LP token staking pool.
    UpdateWeight {
        /// cluster token contract address
        asset_token: String,
        /// weight for the cluster LP token staking pool
        weight: u32,
    },
    /// CreateCluster creates a new cluster along with necessary components.
    CreateCluster {
        /// used to create all necessary contract and register contract token
        params: Params,
    },
    /// DecommissionCluster deactivates an active cluster.
    DecommissionCluster {
        /// cluster contract address
        cluster_contract: String,
        /// cluster token contract address
        cluster_token: String,
    },
    /// PassCommand calls the provided contract to execute the given message.
    PassCommand {
        /// address of a target contract
        contract_addr: String,
        /// message to be executed
        msg: Binary,
    },

    /////////////////////
    /// User Callable
    /////////////////////

    /// Distribute performs reward distribution process.
    Distribute {},
}

/// ## Description
/// This structure describes the available query messages for the factory contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// ClusterExists returns whether the provided address is an active cluster.
    ClusterExists {
        /// address to be queried
        contract_addr: String,
    },
    /// ClusterList returns a list of (cluster contract address, active status).
    ClusterList {},
    /// DistributionInfo returns last reward distributed time and reward weights of
    /// all cluster LP token staking pools
    DistributionInfo {},
}

/// ## Description
// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub nebula_token: String,
    pub staking_contract: String,
    pub commission_collector: String,
    pub protocol_fee_rate: String,
    pub astroport_factory: String,
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>,
}

/// ## Description
/// A custom struct for each query response that returns an active cluster status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterExistsResponse {
    /// whether is an active cluster
    pub exists: bool,
}

/// ## Description
/// A custom struct for each query response that returns a list of
/// pairs of cluster contract addresses and active status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClusterListResponse {
    /// vector of (cluster contract address, active status)
    pub contract_infos: Vec<(String, bool)>,
}

/// ## Description
/// A custom struct for each query response that returns distribution information containing
/// last reward distributed time and weights of cluster LP token staking pools.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    /// reward distribution weights of cluster LP token staking pools
    pub weights: Vec<(String, u32)>,
    /// last reward distributed time
    pub last_distributed: u64,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// ## Description
/// A custom struct for storing factory parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Params {
    /// Name of cluster
    pub name: String,

    /// Symbol of cluster
    pub symbol: String,

    /// Description of cluster
    pub description: String,

    /// Distribution weight (default is 30, which is 1/10 of NEB distribution weight)
    pub weight: Option<u32>,

    /// Corresponding penalty contract to query for mint/redeem
    pub penalty: Addr,

    /// Pricing oracle address
    pub pricing_oracle: Addr,

    /// Composition oracle address
    pub target_oracle: Addr,

    /// Target assets and weights
    pub target: Vec<Asset>,
}
