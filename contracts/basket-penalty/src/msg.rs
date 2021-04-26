use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    SetPrices { prices: Vec<(String, Decimal)> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Mint {
        inventory: Vec<Uint128>,
        mint_asset_amounts: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<Uint128>
    },

    Redeem {
        inventory: Vec<Uint128>,
        redeem_tokens: Uint128,
        redeem_weights: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<Uint128>,
    }
}

#[derive(Serialize, Deserialize)]
pub struct MintResponse {
    pub mint_tokens: Uint128,
}

#[derive(Serialize, Deserialize)]
pub struct RedeemResponse {
    pub redeem_assets: Vec<Uint128>,
}
