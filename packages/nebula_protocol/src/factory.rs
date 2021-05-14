use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, HumanAddr, Uint128};

use terraswap::asset::{AssetInfo, Asset};
use cw20::Cw20ReceiveMsg;
use terraswap::hook::InitHook;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub protocol_fee_rate: String,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    PostInitialize {
        owner: HumanAddr,
        terraswap_factory: HumanAddr,
        nebula_token: HumanAddr,
        staking_contract: HumanAddr,
        oracle_contract: HumanAddr,
        commission_collector: HumanAddr,
    },
    UpdateConfig {
        owner: Option<HumanAddr>,
        token_code_id: Option<u64>,
        cluster_code_id: Option<u64>,
        distribution_schedule: Option<Vec<(u64, u64, Uint128)>>, // [[start_time, end_time, distribution_amount], [], ...]
    },
    UpdateWeight {
        asset_token: HumanAddr,
        weight: u32,
    },
    CreateCluster {
        /// asset name used to create token contract
        name: String,
        /// asset symbol used to create token contract
        symbol: String,
        /// used to create all necessary contract or register asset
        params: Params,
    },
    /// Internal use
    TokenCreationHook {},
    /// Internal use
    TerraswapCreationHook {
        asset_token: HumanAddr,
    },
    /// Internal use - Set Basket Token
    SetBasketTokenHook {
        cluster: HumanAddr,
    },
    PassCommand {
        contract_addr: HumanAddr,
        msg: Binary,
    },

    //////////////////////
    // Feeder Operations
    //////////////////////

    // Revoke asset from MIR rewards pool
    // and register end_price to mint contract
    // RevokeAsset {
    //     asset_token: HumanAddr,
    //     end_price: Decimal,
    // },
    // Migrate asset to new asset by registering
    // end_price to mint contract and add
    // the new asset to MIR rewards pool
    // MigrateAsset {
    //     name: String,
    //     symbol: String,
    //     from_token: HumanAddr,
    //     end_price: Decimal,
    // },

    ///////////////////
    ///////////////////
    Distribute {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    // DistributionInfo {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub nebula_token: HumanAddr,
    pub staking_contract: HumanAddr,
    pub commission_collector: HumanAddr,
    pub protocol_fee_rate: String,
    pub oracle_contract: HumanAddr,
    pub terraswap_factory: HumanAddr,
    pub token_code_id: u64,
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    pub weights: Vec<(HumanAddr, u32)>,
    pub last_distributed: u64,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Params {
    // Name of basket 
    pub name: String,

    // Symbol of basket
    pub symbol: String,
    /// Distribution weight (default is 30, which is 1/10 of NEB distribution weight)
    pub weight: Option<u32>,

    // Corresponding penalty contract to query for mint/redeem
    pub penalty: HumanAddr,

    /// Oracle address
    pub oracle: HumanAddr,
    
    pub assets: Vec<AssetInfo>,

    pub target: Vec<u32>
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BasketHandleMsg {
    Receive(Cw20ReceiveMsg),

    /// Withdraws asset from staging
    UnstageAsset {
        asset: AssetInfo,
        amount: Option<Uint128>,
    },

    /// Stages native asset
    StageNativeAsset {
        asset: Asset,
    },

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
    // AddAssetType {
    //     asset: HumanAddr,
    // },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BasketInitMsg {
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
pub enum StakingHandleMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        owner: Option<HumanAddr>,
        premium_min_update_interval: Option<u64>,
    },
    RegisterAsset {
        asset_token: HumanAddr,
        staking_token: HumanAddr,
    },

    ////////////////////////
    /// User operations ///
    ////////////////////////
    Unbond {
        asset_token: HumanAddr,
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        // If the asset token is not given, then all rewards are withdrawn
        asset_token: Option<HumanAddr>,
    },

    //////////////////////////////////
    /// Permission-less operations ///
    //////////////////////////////////
    AdjustPremium {
        asset_tokens: Vec<HumanAddr>,
    },

    ////////////////////////////////
    /// Mint contract operations ///
    ////////////////////////////////
    IncreaseShortToken {
        asset_token: HumanAddr,
        staker_addr: HumanAddr,
        amount: Uint128,
    },
    DecreaseShortToken {
        asset_token: HumanAddr,
        staker_addr: HumanAddr,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StakingCw20HookMsg {
    Bond { asset_token: HumanAddr },
    DepositReward { rewards: Vec<(HumanAddr, Uint128)> },
}