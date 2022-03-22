use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::AssetInfo;

/// ## Description
/// This structure stores the basic settings for creating a new oracle contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the contract
    pub owner: String,
    /// TeFi oracle hub contract
    pub oracle_addr: String,
    /// Default denom, UST (uusd)
    pub base_denom: String,
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
        owner: Option<String>,
        /// new TeFi oracle hub contract
        oracle_addr: Option<String>,
        /// new default denom
        base_denom: Option<String>,
    },
}

/// ## Description
/// This structure describes the available query messages for the oracle contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// Price returns the latest oracle price of `base_asset` in `quote_asset` unit.
    Price {
        /// an asset to be queried
        base_asset: AssetInfo,
        /// an asset used as a price unit
        quote_asset: AssetInfo,
    },
}

/// ## Description
// A custom struct for each query response that returns the latest oracle price of an asset.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PriceResponse {
    /// Price of the base asset in quote asset unit
    pub rate: Decimal,
    /// Last update time of the base asset
    pub last_updated_base: u64,
    /// Last update time of the quote asset
    pub last_updated_quote: u64,
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the oracle contract
    pub owner: String,
    /// TeFi Oracle Hub contract
    pub oracle_addr: String,
    /// Base denom, UST
    pub base_denom: String,
}
