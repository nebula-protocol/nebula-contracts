use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub distribution_contract: HumanAddr, // collected rewards receiver
    pub terraswap_factory: HumanAddr,
    pub nebula_token: HumanAddr,
    pub base_denom: String,
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// USER-CALLABLE
    Convert { asset_token: HumanAddr },
    Distribute {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub distribution_contract: HumanAddr, // collected rewards receiver
    pub terraswap_factory: HumanAddr,
    pub nebula_token: HumanAddr,
    pub base_denom: String,
    pub owner: HumanAddr,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
