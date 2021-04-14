use cosmwasm_std::StdError;
use std::str::FromStr;
use bigint::U256;

use crate::fp_decimal::FPDecimal;

impl FromStr for FPDecimal {
    type Err = StdError;

    /// Converts the decimal string to a Decimal256
    /// Possible inputs: "1.23", "1", "000012", "1.123000000"
    /// Disallowed: "", ".23"
    ///
    /// This never performs any kind of rounding.
    /// More than 18 fractional digits, even zeros, result in an error.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let sign = if input.starts_with('-') { 0 } else { 1 };
        let num = U256::from_dec_str(&input[1..])
            .map_err(|_| StdError::generic_err("Error parsing integer"))?;
        Ok(FPDecimal {num: num, sign: sign})
    }
}
