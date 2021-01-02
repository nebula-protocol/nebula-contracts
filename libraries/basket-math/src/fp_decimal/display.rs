use crate::fp_decimal::{FPDecimal, Sign};
use std::fmt;

impl fmt::Display for FPDecimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.sign() == Sign::Negative {
            "-"
        } else {
            ""
        };
        let integer = (FPDecimal::_int(self.0) / FPDecimal::ONE).abs();
        let fraction = (FPDecimal::_fraction(self.0)).abs();

        if fraction == 0 {
            write!(f, "{}{}", sign, integer.to_string())
        } else {
            let fraction_string = fraction.to_string();
            let fraction_string =
                "0".repeat(FPDecimal::DIGITS - fraction_string.len()) + &fraction_string;
            f.write_str(sign)?;
            f.write_str(&integer.to_string())?;
            f.write_str(".")?;
            f.write_str(fraction_string.trim_end_matches('0'))?;

            Ok(())
        }
    }
}
