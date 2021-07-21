use cosmwasm_std::{Decimal, StdResult};

const DECIMAL_FRACTION: u128 = 1_000_000_000_000_000_000u128;
pub fn decimal_subtraction(a: Decimal, b: Decimal) -> StdResult<Decimal> {
    Ok(Decimal::from_ratio(
        (a * DECIMAL_FRACTION.into() - b * DECIMAL_FRACTION.into())?,
        DECIMAL_FRACTION,
    ))
}

pub fn decimal_inverse(a: Decimal) -> Decimal {
    Decimal::from_ratio(
        Decimal::one() * DECIMAL_FRACTION.into(),
        a * DECIMAL_FRACTION.into(),
    )
}
