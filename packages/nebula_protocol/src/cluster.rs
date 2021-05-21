use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};
use terraswap::hook::InitHook;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Basket name (title)
    pub name: String,

    /// Basket's permissioned owner
    pub owner: HumanAddr,

    /// Basket token CW20 address
    pub basket_token: Option<HumanAddr>,

    /// Asset addresses
    pub assets: Vec<AssetInfo>,

    /// Factory address
    pub factory: HumanAddr,

    /// Oracle address
    pub oracle: HumanAddr,

    /// Penalty function address
    pub penalty: HumanAddr,

    /// Target weight vector (not normalized)
    pub target: Vec<u32>,

    pub init_hook: Option<InitHook>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Called to set basket token after initialization
    _SetBasketToken {
        basket_token: HumanAddr,
    },

    /// Can be called by the owner to reset the basket owner
    _ResetOwner {
        owner: HumanAddr,
    },

    /// Can be called by the owner to reset the basket weight target
    ResetTarget {
        assets: Vec<AssetInfo>,
        target: Vec<u32>,
    },

    ResetPenalty {
        penalty: HumanAddr,
    },

    /// Mints new assets
    Mint {
        /// Asset amounts deposited for minting (must be staged)
        asset_amounts: Vec<Asset>,
        /// Minimum tokens to receive
        min_tokens: Option<Uint128>,
    },
    /// Burns assets
    Burn {
        /// optional proposed set of weights to use
        max_tokens: Uint128,
        asset_amounts: Option<Vec<Asset>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Target {},
    BasketState {
        basket_contract_address: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub config: BasketConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TargetResponse {
    pub assets: Vec<AssetInfo>,
    pub target: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketStateResponse {
    pub outstanding_balance_tokens: Uint128,
    pub prices: Vec<String>,
    pub inv: Vec<Uint128>,
    pub assets: Vec<AssetInfo>,
    pub penalty: HumanAddr,
    pub target: Vec<u32>,
    pub basket_contract_address: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketConfig {
    pub name: String,
    pub owner: HumanAddr,
    pub basket_token: Option<HumanAddr>,
    pub factory: HumanAddr,
    pub oracle: HumanAddr,
    pub penalty: HumanAddr,
}