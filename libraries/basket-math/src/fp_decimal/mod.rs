#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FPDecimal(pub i128);

#[derive(PartialEq)]
pub enum Sign {
    Positive,
    Negative,
    NoSign,
}

impl FPDecimal {
    pub const MAX: i128 = 170141183460469231731687303715884105727i128;
    pub const MIN: i128 = -170141183460469231731687303715884105727i128;
    pub const DIGITS: usize = 18;
    pub const ONE: i128 = 1_000_000_000_000_000_000;
    pub const MUL_PRECISION: i128 = 1_000_000_000;
    pub const E_10: i128 = 22026465794806716516958; // e^10
    pub const E: i128 = 2718281828459045235; // e
    pub const LN_10: i128 = 2302585092994045684; // ln(10)
    pub const LN_1_5: i128 = 405465108108164382; // ln(1.5)

    pub const fn one() -> FPDecimal {
        FPDecimal(FPDecimal::ONE)
    }

    pub const fn zero() -> FPDecimal {
        FPDecimal(0i128)
    }

    pub const fn max() -> FPDecimal {
        FPDecimal(FPDecimal::MAX)
    }

    pub const fn min() -> FPDecimal {
        FPDecimal(FPDecimal::MIN)
    }

    // converts integer into decimal
    pub fn from<T>(item: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        FPDecimal(item.into() * FPDecimal::ONE)
    }

    pub fn _int(x: i128) -> i128 {
        x / FPDecimal::ONE * FPDecimal::ONE
    }

    pub fn _sign(x: i128) -> i128 {
        if x >= 0 {
            1
        } else {
            0
        }
    }

    pub fn sign(&self) -> Sign {
        if self.0 == 0 {
            Sign::NoSign
        } else if self.0 < 0 {
            Sign::Negative
        } else {
            Sign::Positive
        }
    }

    pub fn int(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_int(self.0))
    }

    pub fn _fraction(x: i128) -> i128 {
        x - FPDecimal::_int(x)
    }

    pub fn fraction(&self) -> FPDecimal {
        FPDecimal(FPDecimal::_fraction(self.0))
    }
}

mod arithmetic;
mod display;
mod exp;
mod from_str;
mod hyper;
mod log;
