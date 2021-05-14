use cosmwasm_std::{LogAttribute, Uint128, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use basket_math::FPDecimal;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub owner: HumanAddr,
    pub penalty_params: PenaltyParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Can be called by the owner to reset the basket owner
    _ResetOwner {
        owner: HumanAddr,
    },

    Mint {
        block_height: u64,
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        mint_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },

    Redeem {
        block_height: u64,
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        max_tokens: Uint128,
        redeem_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Mint {
        block_height: u64,
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        mint_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },

    Redeem {
        block_height: u64,
        basket_token_supply: Uint128,
        inventory: Vec<Uint128>,
        max_tokens: Uint128,
        redeem_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<u32>,
    },

    Params {

    }
}

#[derive(Serialize, Deserialize)]
pub struct MintResponse {
    pub mint_tokens: Uint128,
    pub log: Vec<LogAttribute>,
}

#[derive(Serialize, Deserialize)]
pub struct RedeemResponse {
    pub redeem_assets: Vec<Uint128>,
    pub token_cost: Uint128,
    pub log: Vec<LogAttribute>,
}

#[derive(Serialize, Deserialize)]
pub struct ParamsResponse {
    pub penalty_params: PenaltyParams,
}

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