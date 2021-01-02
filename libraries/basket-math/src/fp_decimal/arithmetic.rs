/// Arithmetic operators for FPDecimal
use crate::fp_decimal::FPDecimal;
use std::ops;

impl FPDecimal {
    pub fn _add(x: i128, y: i128) -> i128 {
        x + y
    }

    pub fn add<T>(&self, other: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        *self + FPDecimal::from(other)
    }

    pub fn _sub(x: i128, y: i128) -> i128 {
        FPDecimal::_add(x, -y)
    }

    pub fn sub<T>(&self, other: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        *self - FPDecimal::from(other)
    }

    pub fn _mul(x: i128, y: i128) -> i128 {
        let x1: i128 = FPDecimal::_int(x) / FPDecimal::ONE;
        let mut x2: i128 = FPDecimal::_fraction(x);
        let y1: i128 = FPDecimal::_int(y) / FPDecimal::ONE;
        let mut y2: i128 = FPDecimal::_fraction(y);
        let mut x1y1 = x1 * y1;
        let FPDecimal_x1y1 = x1y1 * FPDecimal::ONE;
        x1y1 = FPDecimal_x1y1;
        let x2y1 = x2 * y1;
        let x1y2 = x1 * y2;
        x2 = x2 / FPDecimal::MUL_PRECISION;
        y2 = y2 / FPDecimal::MUL_PRECISION;
        let x2y2 = x2 * y2;
        let mut result = x1y1;
        result = result + x2y1;
        result = result + x1y2;
        result = result + x2y2;
        result
    }

    pub fn mul<T>(&self, other: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        *self * FPDecimal::from(other)
    }

    pub fn _div(x: i128, y: i128) -> i128 {
        if y == FPDecimal::ONE {
            return x;
        }
        assert!(y != 0);
        FPDecimal::_mul(x, FPDecimal::reciprocal(y))
    }

    pub fn div<T>(&self, other: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        *self / FPDecimal::from(other)
    }

    pub fn reciprocal(x: i128) -> i128 {
        assert!(x != 0);
        FPDecimal::ONE * FPDecimal::ONE / x
    }

    pub fn _int(x: i128) -> i128 {
        x / FPDecimal::ONE * FPDecimal::ONE
    }

    pub fn _fraction(x: i128) -> i128 {
        x - FPDecimal::_int(x)
    }

    pub fn abs(&self) -> FPDecimal {
        FPDecimal(self.0.abs())
    }
}

impl ops::Add for FPDecimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        FPDecimal(FPDecimal::_add(self.0, rhs.0))
    }
}

impl ops::Sub for FPDecimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        FPDecimal(FPDecimal::_sub(self.0, rhs.0))
    }
}

impl ops::Mul for FPDecimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        FPDecimal(FPDecimal::_mul(self.0, rhs.0))
    }
}

impl ops::Div for FPDecimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        FPDecimal(FPDecimal::_div(self.0, rhs.0))
    }
}
