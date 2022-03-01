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
    /// OWNER-CALLABLE
    /// UpdateConfig updates contract setting.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        /// Nebula token contract address
        nebula_token: Option<String>,
    },
    /// RegisterMerkleRoot add a new merkle root.
    RegisterMerkleRoot {
        /// merkle root hash used for validating airdrop claims
        merkle_root: String,
    },

    /// USER-CALLABLE
    /// Claim allows the sender to claim their airdrop.
    Claim {
        /// stage of airdrop to be claimed
        stage: u8,
        /// airdrop amount
        amount: Uint128,
        /// merkle proof of the airdrop for validation
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
        /// stage of the merkle root to be queried
        stage: u8,
    },
    /// LatestStage returns the latest stage with a merkle root registered.
    LatestStage {},
    /// IsClaimed returns whether the address already claimed the airdrop
    /// at the specified stage.
    IsClaimed {
        /// stage of airdrop
        stage: u8,
        /// address of a user
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
    /// stage of the merkle root to be queried
    pub stage: u8,
    /// related merkle root
    pub merkle_root: String,
}

/// ## Description
/// We define a custom struct for each query response that returns the latest stage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LatestStageResponse {
    /// latest stage with a merkle root registered
    pub latest_stage: u8,
}

/// ## Description
/// We define a custom struct for each query response that returns the claim status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsClaimedResponse {
    /// airdrop claim status
    pub is_claimed: bool,
}
