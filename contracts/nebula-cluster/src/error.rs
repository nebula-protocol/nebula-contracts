use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    Generic(String),

    #[error("Cluster must contain valid assets and cannot contain duplicate assets")]
    InvalidAssets {},

    #[error("This cluster is already a decommissioned cluster")]
    ClusterAlreadyDecommissioned {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Number of cluster tokens to be minted is below min_tokens: {0} (would_mint) < {1} (min_tokens)")]
    BelowMinTokens(Uint128, Uint128),

    #[error(
        "Cost of assets in cluster tokens is above max_tokens: {0} (would_cost) > {1} (max_tokens)"
    )]
    AboveMaxTokens(Uint128, Uint128),

    #[error("Associated cluster token has not yet been set")]
    ClusterTokenNotSet {},
}
