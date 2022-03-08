use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new collector contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Collected rewards receiver, Governance contract address
    pub distribution_contract: String,
    /// Astroport factory contract address
    pub astroport_factory: String,
    /// Nebula token contract address
    pub nebula_token: String,
    /// Base denom, UST
    pub base_denom: String,
    /// Owner of the collector contract
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateConfig updates contract setting.
    UpdateConfig {
        distribution_contract: Option<String>,
        astroport_factory: Option<String>,
        nebula_token: Option<String>,
        base_denom: Option<String>,
        owner: Option<String>,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// Convert swaps UST to NEB or any CW20 to UST.
    Convert { asset_token: String },
    /// Send collected fee/rewards to Governance contract.
    Distribute {},
}

/// ## Description
/// This structure describes the available query messages for the collector contract.
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
    /// Collected rewards receiver, Governance contract address
    pub distribution_contract: String,
    /// Astroport factory contract address
    pub astroport_factory: String,
    /// Nebula token contract address
    pub nebula_token: String,
    /// Base denom, UST
    pub base_denom: String,
    /// Owner of the collector contract
    pub owner: String,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
