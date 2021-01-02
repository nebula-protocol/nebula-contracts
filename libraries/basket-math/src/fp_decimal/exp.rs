/// Exponential functions for FPDecimal
use crate::fp_decimal::FPDecimal;

impl FPDecimal {
    // a^b
    pub fn _pow(a: i128, b: i128) -> i128 {
        FPDecimal::_exp(FPDecimal::_mul(FPDecimal::_ln(a), b))
    }

    pub fn pow<T>(&self, other: T) -> Self
    where
        T: Into<i128>,
    {
        FPDecimal(FPDecimal::_pow(self.0, FPDecimal::from(other).0))
    }

    // e^(a)
    pub fn _exp(a: i128) -> i128 {
        let mut x = a;
        let mut r = FPDecimal::ONE;
        while x >= 10 * FPDecimal::ONE {
            x -= 10 * FPDecimal::ONE;
            r = FPDecimal::_mul(r, FPDecimal::E_10);
        }
        if x == FPDecimal::ONE {
            return FPDecimal::_mul(r, FPDecimal::E);
        } else if x == 0 {
            return r;
        }
        let mut tr = FPDecimal::ONE;
        let mut d = tr;
        for i in 1..((2 * FPDecimal::DIGITS + 1) as i128) {
            d = (d * x) / (FPDecimal::ONE * i);
            tr += d;
        }
        FPDecimal::_mul(tr, r)
    }
}
