use crate::fp_decimal::FPDecimal;

pub fn sum(vec: &[FPDecimal]) -> FPDecimal {
    vec.iter().fold(FPDecimal::zero(), |acc, &el| acc + el)
}

pub fn dot(vec: &[FPDecimal], other: &[FPDecimal]) -> FPDecimal {
    let mut sum = FPDecimal::zero();
    for i in 0..vec.len() {
        sum = sum + vec[i] * other[i]
    }
    sum
}

pub fn mul(vec: &[FPDecimal], other: &[FPDecimal]) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 * i2).collect()
}

pub fn mul_const(vec: &[FPDecimal], other: FPDecimal) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i * other).collect()
}

pub fn div_const(vec: &[FPDecimal], other: FPDecimal) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i / other).collect()
}

pub fn add(vec: &[FPDecimal], other: &[FPDecimal]) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 + i2).collect()
}

pub fn sub(vec: &[FPDecimal], other: &[FPDecimal]) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 - i2).collect()
}

pub fn abs(vec: &[FPDecimal]) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i.abs()).collect()
}