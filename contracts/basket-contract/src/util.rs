use basket_math::FPDecimal;
use cosmwasm_std::Uint128;

/// ensures casting u128 -> i128 does not overflow
pub fn cast_u128_i128(x: u128) -> i128 {
    assert!(x < (i128::MAX / 1000) as u128);
    x as i128
}

pub fn cast_i128_u128(x: i128) -> u128 {
    assert!(x >= 0);
    x as u128
}

/// converts integer amounts (for coin balances) into FPDecimal for calculation
pub fn int_to_fpdec(amount: Uint128) -> FPDecimal {
    FPDecimal(cast_u128_i128(amount.u128()) * FPDecimal::ONE)
}

/// converts into integer
pub fn fpdec_to_int(dec: FPDecimal) -> (Uint128, FPDecimal) {
    (
        Uint128(cast_i128_u128(dec.int().0 / FPDecimal::ONE)),
        dec.fraction(),
    )
}

/// Prints vectors
pub fn vec_to_string<T>(v: &Vec<T>) -> String
where
    T: ToString,
{
    let str_vec = v.iter().map(|fp| fp.to_string()).collect::<Vec<String>>();
    format!("[{}]", str_vec.join(", "))
}
