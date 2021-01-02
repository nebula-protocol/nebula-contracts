use basket_math::FPDecimal;
use cosmwasm_std::Uint128;

pub fn checked_cast(x: u128) -> i128 {
    assert!(x < i128::MAX as u128);
    x as i128
}

/// Converts vector of Uint128 to FPDecimals
pub fn to_fpdec_vec(v: &Vec<Uint128>) -> Vec<FPDecimal> {
    v.iter()
        .map(|&x| FPDecimal(checked_cast(x.u128()) * FPDecimal::ONE))
        .collect()
}
