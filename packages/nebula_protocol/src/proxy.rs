use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Uint128};

/// ## Description
/// This structure stores the basic settings for creating a new proxy contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Cluster factory contract
    pub factory: String,
    /// Incentives contract
    pub incentives: Option<String>,
    /// Astroport factory contract
    pub astroport_factory: String,
    /// Nebula token contract
    pub nebula_token: String,
    /// Base denom, UST
    pub base_denom: String,
    /// Owner of the contract
    pub owner: String,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateConfig updates contract owner.
    UpdateConfig {
        /// address to claim the contract ownership
        owner: Option<String>,
        incentives: Option<Option<String>>,
    },

    /////////////////////
    /// INTERNAL
    /////////////////////

    /// _SendAll sends all specified assets to an address.
    _SendAll {
        /// assets to be sent
        asset_infos: Vec<AssetInfo>,
        /// receiver address
        send_to: Addr,
    },
    /// _SwapAll changes either all CT -> UST or UST -> CT.
    _SwapAll {
        /// Astroport pair contract
        astroport_pair: Addr,
        /// cluster token (CT) contract
        cluster_token: Addr,
        /// expected return from the swap
        min_return: Option<Uint128>,
        /// swap direction
        to_ust: bool,
        /// base denom
        base_denom: String,
    },
    /// _InternalRewardedCreate calls the actual create logic in a cluster contract
    /// used in both arbitraging and rebalancing.
    _InternalRewardedCreate {
        /// an address performing the rebalance
        rebalancer: Addr,
        /// cluster contract
        cluster_contract: Addr,
        /// incentives contract
        incentives: Option<Addr>,
        /// asset amounts used to mint CT
        asset_amounts: Vec<Asset>,
        /// minimum amount of CT required from minting
        min_tokens: Option<Uint128>,
    },
    /// _InternalRewardedRedeem calls the actual redeem logic in a cluster contract
    /// used in both arbitraging and rebalancing.
    _InternalRewardedRedeem {
        /// an address performing the rebalance
        rebalancer: Addr,
        /// cluster contract
        cluster_contract: Addr,
        /// cluster token contract
        cluster_token: Addr,
        /// incentives contract
        incentives: Option<Addr>,
        /// maximum amount of CT allowed to be burned
        max_tokens: Option<Uint128>,
        /// asset amounts required from burning CT if specified
        asset_amounts: Option<Vec<Asset>>,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// ArbClusterCreate executes the create operation and uses CT to arbitrage on Astroport.
    ArbClusterCreate {
        /// cluster contract
        cluster_contract: String,
        /// assets offerred for minting
        assets: Vec<Asset>,
        /// minimum returned UST when arbitraging
        min_ust: Option<Uint128>,
    },
    /// ArbClusterRedeem executes arbitrage on Astroport to get CT and perform the redeem operation.
    ArbClusterRedeem {
        /// cluster contract
        cluster_contract: String,
        /// UST amount
        asset: Asset,
        /// minimum returned cluster tokens when arbitraging
        min_cluster: Option<Uint128>,
    },
    /// IncentivesCreate executes the create operation on a specific cluster.
    IncentivesCreate {
        /// cluster contract
        cluster_contract: String,
        /// assets offerred for minting
        asset_amounts: Vec<Asset>,
        /// minimum cluster tokens returned
        min_tokens: Option<Uint128>,
    },
    /// IncentivesRedeem executes the redeem operation on a specific cluster.
    IncentivesRedeem {
        /// cluster contract
        cluster_contract: String,
        /// maximum amount of cluster tokens (CT) allowed to be burned
        max_tokens: Uint128,
        /// specific asset amounts returned from burning cluster tokens
        asset_amounts: Option<Vec<Asset>>,
    },
}

/// ## Description
/// This structure describes the available query messages for the incentives contract.
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
    /// Cluster factory contract
    pub factory: String,
    /// Astroport factory contract
    pub astroport_factory: String,
    /// Nebula token contract
    pub nebula_token: String,
    /// Base denom, UST
    pub base_denom: String,
    /// Owner of the contract
    pub owner: String,
    /// Incentives contract
    pub incentives: Option<String>,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
