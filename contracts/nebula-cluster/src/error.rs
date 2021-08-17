use cosmwasm_std::{StdError, Uint128};
use terraswap::asset::AssetInfo;

pub fn missing_cw20_msg() -> StdError {
    StdError::generic_err("Receive Hook - missing expected .msg in body")
}

pub fn bad_weight_dimensions(provided: usize, expected: usize) -> StdError {
    StdError::generic_err(format!(
        "# assets in weights ({}) does not match cluster inventory ({})",
        provided, expected
    ))
}

pub fn bad_weight_values(provided: u32) -> StdError {
    StdError::generic_err(format!("weights do not add to 100 (given {}", provided))
}

pub fn not_component_cw20(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is not a component asset of cluster",
        asset
    ))
}

pub fn not_component_asset(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is not a component asset of cluster",
        asset
    ))
}

pub fn existing_asset(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is already a component asset of cluster",
        asset
    ))
}

pub fn below_min_tokens(would_mint: Uint128, min_tokens: Uint128) -> StdError {
    StdError::generic_err(format!(
        "# cluster tokens to be minted is below min_tokens specified: {} (would_mint) < {} (min_tokens)",
        would_mint, min_tokens
    ))
}

pub fn above_max_tokens(would_cost: Uint128, max_tokens: Uint128) -> StdError {
    StdError::generic_err(format!(
        "cost of assets in cluster tokens is above max_tokens specified: {} (would_cost) > {} (max_tokens)",
        would_cost, max_tokens
    ))
}

pub fn cluster_token_not_set() -> StdError {
    StdError::generic_err("associated cluster token has not yet been set")
}

pub fn cluster_token_already_set(address: &String) -> StdError {
    StdError::generic_err(format!("cluster token has not yet been set to {}", address))
}

pub fn i128_overflow(x: u128) -> StdError {
    StdError::generic_err(format!("can not convert to i128 (overflow): {}", x))
}

pub fn u128_underflow(x: i128) -> StdError {
    StdError::generic_err(format!("can not convert to u128 (underflow): {}", x))
}
