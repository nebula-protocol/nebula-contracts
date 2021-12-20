use cosmwasm_std::{StdError, Uint128};

pub fn below_min_tokens(would_mint: Uint128, min_tokens: Uint128) -> StdError {
    StdError::generic_err(format!(
        "Number of cluster tokens to be minted is below min_tokens: {} (would_mint) < {} (min_tokens)",
        would_mint, min_tokens
    ))
}

pub fn above_max_tokens(would_cost: Uint128, max_tokens: Uint128) -> StdError {
    StdError::generic_err(format!(
        "Cost of assets in cluster tokens is above max_tokens: {} (would_cost) > {} (max_tokens)",
        would_cost, max_tokens
    ))
}

pub fn cluster_token_not_set() -> StdError {
    StdError::generic_err("Associated cluster token has not yet been set")
}
