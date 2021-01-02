use cosmwasm_std::StdError;
use std::str::FromStr;

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
        let sign = if input.starts_with('-') { -1 } else { 1 };
        let parts: Vec<&str> = input.split('.').collect();
        match parts.len() {
            1 => {
                let integer = i128::from_str(parts[0])
                    .map_err(|_| StdError::generic_err("Error parsing integer"))?;
                Ok(FPDecimal::from(sign * integer.abs()))
            }
            2 => {
                let integer = i128::from_str(parts[0])
                    .map_err(|_| StdError::generic_err("Error parsing integer"))?;
                let fraction = i128::from_str(parts[1])
                    .map_err(|_| StdError::generic_err("Error parsing fraction"))?;
                let exp = FPDecimal::DIGITS
                    .checked_sub(parts[1].len())
                    .ok_or_else(|| {
                        StdError::generic_err(format!(
                            "Cannot parse more than {} fractional digits",
                            FPDecimal::DIGITS
                        ))
                    })?;
                Ok(FPDecimal(
                    sign * (integer.abs() * FPDecimal::ONE + fraction * 10i128.pow(exp as u32)),
                ))
            }
            _ => Err(StdError::generic_err("Unexpected number of dots")),
        }
    }
}
