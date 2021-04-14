/// Exponential functions for FPDecimal
use crate::fp_decimal::{FPDecimal, U256};

impl FPDecimal {
    // a^b
    pub fn _pow(a: FPDecimal, b: FPDecimal) -> FPDecimal {
        FPDecimal::_exp(FPDecimal::_mul(FPDecimal::_ln(a), b))
    }

    // e^(a)
    pub fn _exp(a: FPDecimal) -> FPDecimal {
        let mut x = a.num;
        let mut r = FPDecimal::ONE.num;
        while x >= U256([10, 0, 0, 0]) * FPDecimal::ONE.num {
            x = x - U256([10, 0, 0, 0]) * FPDecimal::ONE.num;
            r = FPDecimal::_mul(FPDecimal {num: r, sign: 1}, FPDecimal::E_10).num;
        }
        if x == FPDecimal::ONE.num {
            let val = FPDecimal::_mul(FPDecimal {num: r, sign: 1}, FPDecimal::E);
            if a.sign == 0 {
                return FPDecimal::reciprocal(val);
            }
            return val;
        } else if x == FPDecimal::zero().num {
            let val = FPDecimal {num: r, sign: 1};
            if a.sign == 0 {
                return FPDecimal::reciprocal(val);
            }
            return val;
        }
        let mut tr = FPDecimal::ONE.num;
        let mut d = tr;
        for i in 1..((2 * FPDecimal::DIGITS + 1) as u64) {
            d = (d * x) / (FPDecimal::ONE.num * U256([i, 0, 0, 0]));
            tr = tr + d;
        }
        let val = FPDecimal::_mul(FPDecimal {num: tr, sign: 1}, FPDecimal {num: r, sign: 1});
        if a.sign == 0 {
            return FPDecimal::reciprocal(val);
        }
        val
    }
}
