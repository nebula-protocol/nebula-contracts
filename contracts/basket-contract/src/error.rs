use cosmwasm_std::{HumanAddr, StdError, Uint128};
use terraswap::asset::{Asset, AssetInfo};

pub fn missing_cw20_msg() -> StdError {
    StdError::generic_err("Receive Hook - missing expected .msg in body")
}

pub fn bad_weight_dimensions(provided: usize, expected: usize) -> StdError {
    StdError::generic_err(format!(
        "# assets in weights ({}) does not match basket inventory ({})",
        provided, expected
    ))
}

pub fn bad_weight_values(provided: u32) -> StdError {
    StdError::generic_err(format!("weights do not add to 100 (given {}", provided))
}

pub fn not_component_cw20(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is not a component asset of basket",
        asset
    ))
}

pub fn not_component_asset(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is not a component asset of basket",
        asset
    ))
}

pub fn existing_asset(asset: &AssetInfo) -> StdError {
    StdError::generic_err(format!(
        "asset {} is already a component asset of basket",
        asset
    ))
}

pub fn insufficient_staged(
    account: &HumanAddr,
    asset: &AssetInfo,
    requested: Uint128,
    staged: Uint128,
) -> StdError {
    StdError::generic_err(format!(
        "account {} - insufficient amount of asset {} to unstage: {} (staged) < {} (requested)",
        account, asset, staged, requested
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

pub fn i128_overflow(x: u128) -> StdError {
    StdError::generic_err(format!("can not convert to i128 (overflow): {}", x))
}

pub fn u128_underflow(x: i128) -> StdError {
    StdError::generic_err(format!("can not convert to u128 (underflow): {}", x))
}
