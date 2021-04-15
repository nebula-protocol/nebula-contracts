use bigint::U256;
use schemars::JsonSchema;
// pub struct FPDecimal(#[schemars(with = "String")] pub i128);

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct FPDecimal {
    #[schemars(with = "String")] pub num: U256,
    pub sign: i8
}

impl From<u128> for FPDecimal {
    fn from(x: u128) -> FPDecimal {
        let second = x >> 64;
        let second_u64 = second as u64;
        FPDecimal {num: U256([x as u64, second_u64 ,0,0]) * FPDecimal::ONE.num, sign: 1}
    }
}

impl From<i128> for FPDecimal {
    fn from(x: i128) -> FPDecimal {
        let mut sign = 1;
        if x < 0 {
            sign = 0;
        }
        let abs_x : u128 = x.abs() as u128;
        let second = abs_x >> 64;
        let second_u64 = second as u64;
        FPDecimal {num: U256([abs_x as u64, second_u64, 0, 0]) * FPDecimal::ONE.num, sign: sign}
    }
}

impl Into<u128> for FPDecimal {
    fn into(self) -> u128 {
        let num = self.int().num / FPDecimal::ONE.num;
        let mut array: [u8; 16] = [0;16];
        for i in 0..16 {
            array[i] = num.byte(i);
        }
        let val = u128::from_le_bytes(array);
        val
    }
}

// #[cfg(not(target_arch = "wasm32"))]
// impl convert::From<FPDecimal> for f32 {
//     fn from(x: FPDecimal) -> f32 {
//         f32::from_str(&x.to_string()).unwrap()
//     }
// }

impl FPDecimal {
    pub const MAX: FPDecimal = FPDecimal {num: U256::MAX, sign: 1};
    pub const MIN: FPDecimal = FPDecimal {num: U256::MAX, sign: 0};
    pub const DIGITS: usize = 18;
    pub const ONE: FPDecimal  = FPDecimal {num: U256([1_000_000_000_000_000_000, 0,0,0]), sign: 1};
    pub const MUL_PRECISION: FPDecimal = FPDecimal {num: U256([1_000_000_000, 0,0,0]), sign: 1};
    pub const E_10: FPDecimal = FPDecimal {num: U256([1053370797511887454u64, 1194u64, 0, 0]), sign:1}; // e^10
    pub const E: FPDecimal = FPDecimal {num: U256([2718281828459045235, 0,0,0]), sign: 1};
    pub const LN_10: FPDecimal = FPDecimal {num: U256([2302585092994045684, 0,0,0]), sign: 1}; // ln(10)
    pub const LN_1_5: FPDecimal = FPDecimal {num: U256([405465108108164382, 0,0,0]), sign: 1}; // ln(1.5)

    pub const fn one() -> FPDecimal {
        FPDecimal::ONE
    }

    pub const fn zero() -> FPDecimal {
        FPDecimal {
            num: U256([0,0,0,0]),
            sign: 1
        }
    }

    pub const fn max() -> FPDecimal {
        FPDecimal::MAX
    }

    pub const fn min() -> FPDecimal {
        FPDecimal::MIN
    }

    pub const fn e() -> FPDecimal {
        FPDecimal::E
    }

    pub fn _int(x: FPDecimal) -> FPDecimal {
        let x1 = x.num;
        let x1_1 = x1 / FPDecimal::ONE.num;
        let x_final = x1_1 * FPDecimal::ONE.num;
        FPDecimal {
            num: x_final,
            sign: x.sign
        }
    }

    pub fn int(&self) -> FPDecimal {
        FPDecimal::_int(*self)
    }

    pub fn _sign(x: FPDecimal) -> i8 {
        x.sign
    }

    pub fn _fraction(x: FPDecimal) -> FPDecimal {

        let x1 = x.num;
        FPDecimal {
            num: x1 - FPDecimal::_int(x).num,
            sign: x.sign
        }
    }

    pub fn fraction(&self) -> FPDecimal {
        FPDecimal::_fraction(*self)
    }
}

mod arithmetic;
mod display;
mod exp;
mod from_str;
mod hyper;
mod log;
mod serde; // cosmwasm serialization