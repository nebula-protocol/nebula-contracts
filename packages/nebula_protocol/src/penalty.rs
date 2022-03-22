use cluster_math::FPDecimal;
use cosmwasm_std::{Attribute, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new penalty contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// owner of the contract
    pub owner: String,
    /// penalty contract parameters
    pub penalty_params: PenaltyParams,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateConfig updates general penalty contract settings.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        /// penalty contract parameters
        penalty_params: Option<PenaltyParams>,
    },

    /// PenaltyCreate updates the state of penalty contract after a create operation.
    PenaltyCreate {
        /// a specific height to compute mint at
        block_height: u64,
        /// current total supply for a cluster token
        cluster_token_supply: Uint128,
        /// current inventory of inventory assets in a cluster
        inventory: Vec<Uint128>,
        /// the provided asset amounts for minting cluster tokens
        create_asset_amounts: Vec<Uint128>,
        /// prices of the inventory assets in a cluster
        asset_prices: Vec<String>,
        /// current target weights of the assets in a cluster
        target_weights: Vec<Uint128>,
    },

    /// PenaltyRedeem updates the state of penalty contract after a redeem operation.
    PenaltyRedeem {
        /// a specific height to compute mint at
        block_height: u64,
        /// current total supply for a cluster token
        cluster_token_supply: Uint128,
        /// current inventory of inventory assets in a cluster
        inventory: Vec<Uint128>,
        /// maximum amount of cluster tokens allowed to burn for pro-rata redeem
        max_tokens: Uint128,
        /// amounts expected to receive from burning cluster tokens
        redeem_asset_amounts: Vec<Uint128>,
        /// prices of the inventory assets in a cluster
        asset_prices: Vec<String>,
        /// current target weights of the assets in a cluster
        target_weights: Vec<Uint128>,
    },
}

/// ## Description
/// This structure describes the available query messages for the penalty contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},

    /// Params returns general contract parameters using a custom [`ParamsResponse`] structure.
    Params {},

    /// PenaltyQueryCreate calculates the actual create amount after taking penalty into consideration.
    PenaltyQueryCreate {
        /// a specific height to compute mint at
        block_height: u64,
        /// current total supply for a cluster token
        cluster_token_supply: Uint128,
        /// current inventory of inventory assets in a cluster
        inventory: Vec<Uint128>,
        /// the provided asset amounts for minting cluster tokens
        create_asset_amounts: Vec<Uint128>,
        /// prices of the inventory assets in a cluster
        asset_prices: Vec<String>,
        /// current target weights of the assets in a cluster
        target_weights: Vec<Uint128>,
    },

    /// PenaltyQueryRedeem calculates the actual redeem amount after taking penalty into consideration.
    PenaltyQueryRedeem {
        /// a specific height to compute mint at
        block_height: u64,
        /// current total supply for a cluster token
        cluster_token_supply: Uint128,
        /// current inventory of inventory assets in a cluster
        inventory: Vec<Uint128>,
        /// maximum amount of cluster tokens allowed to burn for pro-rata redeem
        max_tokens: Uint128,
        /// amounts expected to receive from burning cluster tokens
        redeem_asset_amounts: Vec<Uint128>,
        /// prices of the inventory assets in a cluster
        asset_prices: Vec<String>,
        /// current target weights of the assets in a cluster
        target_weights: Vec<Uint128>,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the contract, cluster contract
    pub owner: String,
    /// General penalty contract parameters
    pub penalty_params: PenaltyParams,
}

/// ## Description
/// A custom struct for each query that returns the actual mint amount and the subjected penalty.
#[derive(Serialize, Deserialize)]
pub struct PenaltyCreateResponse {
    /// Actual minted cluster token amount
    pub create_tokens: Uint128,
    /// Incurred penalty / reward from rebalance
    pub penalty: Uint128,
    /// Returned attributes to the caller
    pub attributes: Vec<Attribute>,
}

/// ## Description
/// A custom struct for each query that returns the actual return assets and the subjected penalty.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PenaltyRedeemResponse {
    /// Actual return assets
    pub redeem_assets: Vec<Uint128>,
    /// Incurred penalty / reward from rebalance
    pub penalty: Uint128,
    /// Actual burned cluster token amount
    pub token_cost: Uint128,
    /// Returned attributes to the caller
    pub attributes: Vec<Attribute>,
}

/// ## Description
/// A custom struct for each query that returns contract state and parameters.
#[derive(Serialize, Deserialize)]
pub struct ParamsResponse {
    /// General penalty contract parameters
    pub penalty_params: PenaltyParams,
    /// Last rebalanced block
    pub last_block: u64,
    /// Last rebalanced EMA
    pub ema: String,
}

/// ## Description
/// A custom struct storing general penalty contract parameters.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
pub struct PenaltyParams {
    // penalty_amt_lo -> amount of penalty when imbalance <= penalty_cutoff_lo * E
    pub penalty_amt_lo: FPDecimal,
    pub penalty_cutoff_lo: FPDecimal,

    // penalty_amt_hi -> amount of penalty when imbalance >= penalty_cutoff_hi * E
    pub penalty_amt_hi: FPDecimal,
    pub penalty_cutoff_hi: FPDecimal,
    // in between penalty_cutoff_hi and penalty_cutoff_lo, the amount of penalty increases linearly

    // reward_amt -> amount of reward when imbalance >= reward_cutoff * E
    // no reward everywhere else
    pub reward_amt: FPDecimal,
    pub reward_cutoff: FPDecimal,
}
