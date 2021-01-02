/// Hyperbolic Trig functions for FPDecimal
use crate::fp_decimal::FPDecimal;

impl FPDecimal {
    pub fn _sinh(x: i128) -> i128 {
        let neg_x: i128 = -1 * x;
        let denominator: i128 = FPDecimal::ONE * 2;
        let numerator: i128 = FPDecimal::_sub(FPDecimal::_exp(x), FPDecimal::_exp(neg_x));
        FPDecimal::_div(numerator, denominator)
    }

    pub fn sinh(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_sinh(self.0))
    }

    pub fn _cosh(x: i128) -> i128 {
        let neg_x: i128 = -1 * x;
        let denominator: i128 = FPDecimal::ONE * 2;
        let numerator: i128 = FPDecimal::_add(FPDecimal::_exp(x), FPDecimal::_exp(neg_x));
        FPDecimal::_div(numerator, denominator)
    }

    pub fn cosh(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_cosh(self.0))
    }

    pub fn _tanh(x: i128) -> i128 {
        FPDecimal::_div(FPDecimal::_sinh(x), FPDecimal::_cosh(x))
    }

    pub fn tanh(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_tanh(self.0))
    }
}
