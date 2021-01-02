use cosmwasm_std::{HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::PenaltyParams;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Contract owner
    pub owner: HumanAddr,

    /// Basket token CW20 address
    pub basket_token: HumanAddr,

    /// Asset addresses
    pub assets: Vec<HumanAddr>,

    /// Oracle address
    pub oracle: HumanAddr,

    /// Penalty function params
    pub penalty_params: PenaltyParams,

    /// Target weight vector (not normalized)
    pub target: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),
    UnstageAsset {
        asset: HumanAddr,
        amount: Uint128,
    },
    ResetTarget {
        target: Vec<u32>,
    },
    Mint {
        /// Asset amounts deposited for minting
        asset_amounts: Vec<Uint128>,
        /// Minimum tokens to receive
        min_tokens: Option<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    StageAsset {
        asset: HumanAddr,
        amount: Uint128,
    },
    Burn {
        num_tokens: Uint128,
        asset_weights: Option<Vec<u32>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTarget {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetResponse {
    pub target: Vec<u32>,
}
