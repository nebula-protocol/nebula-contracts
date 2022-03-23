use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new airdrop contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the airdrop contract
    pub owner: String,
    /// Address string of Nebula token contract
    pub nebula_token: String,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateConfig updates contract setting.
    UpdateConfig {
        /// Address to claim the contract ownership
        owner: Option<String>,
        /// Nebula token contract address
        nebula_token: Option<String>,
    },
    /// RegisterMerkleRoot add a new merkle root.
    RegisterMerkleRoot {
        /// Merkle root hash used for validating airdrop claims
        merkle_root: String,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// Claim allows the sender to claim their airdrop.
    Claim {
        /// Stage of airdrop to be claimed
        stage: u8,
        /// Airdrop amount
        amount: Uint128,
        /// Merkle proof of the airdrop for validation
        proof: Vec<String>,
    },
}

/// ## Description
/// This structure describes the available query messages for the airdrop contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// MerkleRoot returns the registered merkle root at the specified stage.
    MerkleRoot {
        /// Stage of the merkle root to be queried
        stage: u8,
    },
    /// LatestStage returns the latest stage with a merkle root registered.
    LatestStage {},
    /// IsClaimed returns whether the address already claimed the airdrop
    /// at the specified stage.
    IsClaimed {
        /// Stage of airdrop
        stage: u8,
        /// Address of a user
        address: String,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the airdrop contract
    pub owner: String,
    /// Address string of Nebula token contract
    pub nebula_token: String,
}

/// ## Description
/// We define a custom struct for each query response that returns a stage and a related merkle root.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRootResponse {
    /// Stage of the merkle root to be queried
    pub stage: u8,
    /// Related merkle root
    pub merkle_root: String,
}

/// ## Description
/// We define a custom struct for each query response that returns the latest stage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestStageResponse {
    /// Latest stage with a merkle root registered
    pub latest_stage: u8,
}

/// ## Description
/// We define a custom struct for each query response that returns the claim status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsClaimedResponse {
    /// Airdrop claim status
    pub is_claimed: bool,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
