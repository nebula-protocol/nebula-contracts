pub mod fp_decimal;
pub mod vector;

pub use fp_decimal::*;
pub use vector::*;
use std::str::FromStr;
use cosmwasm_std::{StdResult, Uint128};

pub fn imbalance(i: &[FPDecimal], p: &[FPDecimal], w: &[FPDecimal]) -> FPDecimal {
    let wp =  dot(w, p);
    let u = mul(w, p);
    let err_portfolio = sub(&mul_const(&u, dot(i, p)), &mul_const(&mul(i, p), wp));

    sum(&abs(&err_portfolio)) / wp
}

pub fn int32_vec_to_fpdec(arr: &[u32]) -> Vec<FPDecimal> {
    arr.iter()
        .map(|val| FPDecimal::from(*val as u128))
        .collect()
}

pub fn int_vec_to_fpdec(arr: &[Uint128]) -> Vec<FPDecimal> {
    arr.iter().map(|val| FPDecimal::from(val.u128())).collect()
}

pub fn str_vec_to_fpdec(arr: &[String]) -> StdResult<Vec<FPDecimal>> {
    arr.iter()
        .map(|val| FPDecimal::from_str(val))
        .collect::<StdResult<Vec<FPDecimal>>>()
}