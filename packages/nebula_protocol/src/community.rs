use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::Asset;
use cosmwasm_std::Binary;

/// ## Description
/// This structure stores the basic settings for creating a new collector contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner address, Nebula Governance contract
    pub owner: String,
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
        /// address to claim the contract ownership
        owner: String,
    },
    /// Spend sends `asset` to `recipient`.
    Spend {
        /// asset info and amount to send
        asset: Asset,
        /// recipient address
        recipient: String,
    },
    /// PassCommand lets the community contract executes the given command.
    PassCommand {
        /// address of the target contract
        contract_addr: String,
        /// command / message to be executed
        msg: Binary,
    },
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// ## Description
/// This structure describes the available query messages for the community contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
}
