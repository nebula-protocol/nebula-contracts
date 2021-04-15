use basket_math::FPDecimal;
use cosmwasm_std::{StdResult, Uint128};

/// transfer token

/// ensures casting u128 -> i128 does not overflow
// pub fn cast_u128_i128(x: u128) -> StdResult<i128> {
//     if x > (i128::MAX / 1000i128) as u128 {
//         return Err(error::i128_overflow(x));
//     }
//     Ok(x as i128)
// }

// pub fn cast_i128_u128(x: i128) -> StdResult<u128> {
//     if x < 0 {
//         return Err(error::u128_underflow(x));
//     }
//     Ok(x as u128)
// }

/// converts integer amounts (for coin balances) into FPDecimal for calculation
pub fn int_to_fpdec(amount: Uint128) -> StdResult<FPDecimal> {
    Ok(FPDecimal::from(amount.u128()))
}

/// converts into integer
pub fn fpdec_to_int(dec: FPDecimal) -> StdResult<(Uint128, FPDecimal)> {
    let dec_u128 : u128 = dec.into();
    Ok((
        Uint128::from(dec_u128),
        dec.fraction()
    ))
}

/// Prints vectors
pub fn vec_to_string<T>(v: &Vec<T>) -> String
where
    T: ToString,
{
    let str_vec = v.iter().map(|fp| fp.to_string()).collect::<Vec<String>>();
    format!("[{}]", str_vec.join(", "))
}
