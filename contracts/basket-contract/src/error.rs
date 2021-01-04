use cosmwasm_std::{HumanAddr, StdError, Uint128};

pub fn missing_cw20_msg() -> StdError {
    StdError::generic_err("Receive Hook - missing expected .msg in body")
}

pub fn bad_weight_dimensions(provided: usize, expected: usize) -> StdError {
    StdError::generic_err(format!(
        "# assets in weights ({}) does not match basket inventory ({})",
        provided, expected
    ))
}

pub fn not_component_asset(asset: &HumanAddr) -> StdError {
    StdError::generic_err(format!(
        "asset {} is not a component asset of basket",
        asset
    ))
}

pub fn insufficient_staged(asset: &HumanAddr, requested: Uint128, staged: Uint128) -> StdError {
    StdError::generic_err(format!(
        "insufficient amount of asset {} to unstage: {} (staged) < {} (requested)",
        asset, staged, requested
    ))
}

pub fn below_min_tokens(would_mint: Uint128, min_tokens: Uint128) -> StdError {
    StdError::generic_err(format!(
        "# basket tokens to be minted is below min_tokens specified: {} (would_mint) < {} (min_tokens)",
        would_mint, min_tokens
    ))
}

pub fn basket_token_not_set() -> StdError {
    StdError::generic_err("associated basket token has not yet been set")
}

pub fn basket_token_already_set(address: &HumanAddr) -> StdError {
    StdError::generic_err(format!("basket token has not yet been set to {}", address))
}
