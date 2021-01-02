/// Logarithmic functions for FPDecimal
use crate::fp_decimal::FPDecimal;

impl FPDecimal {
    /// natural logarithm
    pub fn _ln(a: i128) -> i128 {
        assert!(a >= 0);
        let mut v = a;
        let mut r = 0;
        while v <= FPDecimal::ONE / 10 {
            v = v * 10;
            r -= FPDecimal::LN_10;
        }
        while v >= 10 * FPDecimal::ONE {
            v = v / 10;
            r += FPDecimal::LN_10;
        }
        while v < FPDecimal::ONE {
            v = FPDecimal::_mul(v, FPDecimal::E);
            r -= FPDecimal::ONE;
        }
        while v > FPDecimal::E {
            v = FPDecimal::_div(v, FPDecimal::E);
            r += FPDecimal::ONE;
        }
        if v == FPDecimal::ONE {
            return r;
        }
        if v == FPDecimal::E {
            return FPDecimal::ONE + r;
        }
        v = v - 3 * FPDecimal::ONE / 2;
        r = r + FPDecimal::LN_1_5;
        let mut m = FPDecimal::ONE * v / (v + 3 * FPDecimal::ONE);
        r = r + 2 * m;
        let m2 = m * m / FPDecimal::ONE;
        let mut i: i128 = 3;
        loop {
            m = m * m2 / FPDecimal::ONE;
            r = r + 2 * m / i;
            i += 2;
            if i >= 3 + 2 * FPDecimal::DIGITS as i128 {
                break;
            }
        }
        r
    }

    pub fn ln(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_ln(self.0))
    }
}

#[cfg(test)]
mod tests {

    use crate::FPDecimal;

    #[test]
    fn test_ln() {
        assert_eq!(FPDecimal::_ln(FPDecimal::E), FPDecimal::ONE);
    }
}
