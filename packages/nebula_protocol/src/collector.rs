use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub distribution_contract: String, // collected rewards receiver
    pub terraswap_factory: String,
    pub nebula_token: String,
    pub base_denom: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// USER-CALLABLE
    Convert {
        asset_token: String,
    },
    Distribute {},
    UpdateConfig {
        distribution_contract: Option<String>,
        terraswap_factory: Option<String>,
        nebula_token: Option<String>,
        base_denom: Option<String>,
        owner: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub distribution_contract: String, // collected rewards receiver
    pub terraswap_factory: String,
    pub nebula_token: String,
    pub base_denom: String,
    pub owner: String,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
