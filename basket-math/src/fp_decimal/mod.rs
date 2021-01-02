#[derive(Copy, Clone)]
pub struct FPDecimal(pub i128);

impl FPDecimal {
    pub const DIGITS: i128 = 18;
    pub const ONE: i128 = 1_000_000_000_000_000_000;
    pub const MUL_PRECISION: i128 = 1_000_000_000;
    pub const E_10: i128 = 22026465794806716516958; // rounded up last digit to 8
    pub const E: i128 = 2718281828459045235;
    pub const LN_10: i128 = 2302585092994045684; // ln(10)
    pub const LN_1_5: i128 = 405465108108164382; // ln(1.5)

    pub const fn one() -> FPDecimal {
        FPDecimal(FPDecimal::ONE)
    }

    pub const fn zero() -> FPDecimal {
        FPDecimal(0i128)
    }

    // converts integer into decimal
    pub fn from<T>(item: T) -> FPDecimal
    where
        T: Into<i128>,
    {
        FPDecimal(item.into() * FPDecimal::ONE)
    }
}

mod arithmetic;
mod exp;
mod hyper;
mod log;
