use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new incentives custody contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// Nebula token contract
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

    /// UpdateOwner updates the config of the contract.
    UpdateConfig {
        /// new owner of the contract
        owner: String,
    },
    /// RequestNeb sends Nebula tokens to the message sender.
    RequestNeb {
        /// amount of Nebula token requested
        amount: Uint128,
    },
}

/// ## Description
/// This structure describes the available query messages for the incentives custody contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// Balance returns the current Nebula token balance of the incentives custody contract.
    Balance {},
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the airdrop contract
    pub owner: String,
}
