/// Logarithmic functions for FPDecimal
use crate::fp_decimal::{FPDecimal, U256};

impl FPDecimal {
    /// natural logarithm
    pub fn _ln(a: FPDecimal) -> FPDecimal {
        assert!(a.sign != 0);
        let mut v = a.num;
        let mut r = FPDecimal::zero().num;
        while v <= FPDecimal::ONE.num / U256([10, 0, 0, 0]) {
            v = v * U256([10, 0, 0, 0]);
            r = r - FPDecimal::LN_10.num;
        }
        while v >= U256([10, 0, 0, 0]) * FPDecimal::ONE.num {
            v = v / U256([10, 0, 0, 0]);
            r = r + FPDecimal::LN_10.num;
        }
        while v < FPDecimal::ONE.num {
            v = FPDecimal::_mul(FPDecimal {num: v, sign: 1}, FPDecimal::E).num;
            r = r - FPDecimal::ONE.num;
        }
        while v > FPDecimal::E.num {
            v = FPDecimal::_div(FPDecimal {num: v, sign: 1}, FPDecimal::E).num;
            r = r + FPDecimal::ONE.num;
        }
        if v == FPDecimal::ONE.num {
            return FPDecimal{num: r, sign: 1};
        }
        if v == FPDecimal::E.num {
            return FPDecimal {num: FPDecimal::ONE.num + r, sign: 1};
        }
        v = v - U256([3, 0, 0, 0]) * FPDecimal::ONE.num / U256([2, 0, 0, 0]);
        r = r + FPDecimal::LN_1_5.num;
        let mut m = FPDecimal::ONE.num * v / (v + U256([3, 0, 0, 0]) * FPDecimal::ONE.num);
        r = r + U256([2, 0, 0, 0]) * m;
        let m2 = m * m / FPDecimal::ONE.num;
        let mut i: u64 = 3;
        loop {
            m = m * m2 / FPDecimal::ONE.num;
            r = r + U256([2, 0, 0, 0]) * m / U256([i, 0, 0, 0]);
            i += 2;
            if i >= 3 + 2 * FPDecimal::DIGITS as u64 {
                break;
            }
        }
        FPDecimal {num: r, sign: 1}
    }

    pub fn ln(&self) -> FPDecimal {
        FPDecimal::_ln(*self)
    }
}

#[cfg(test)]
mod tests {

    use crate::FPDecimal;
    use bigint::U256;
    

    #[test]
    fn test_ln() {
        assert_eq!(FPDecimal::_ln(FPDecimal::E), FPDecimal::ONE);
    }

    #[test]

    fn test_ln10() {
        assert_eq!(FPDecimal::_ln(FPDecimal::E_10), FPDecimal {num: FPDecimal::ONE.num * U256([10, 0, 0, 0]), sign: 1});
    }
}
