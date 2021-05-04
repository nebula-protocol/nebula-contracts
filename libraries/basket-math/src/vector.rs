use crate::fp_decimal::FPDecimal;

pub fn sum(vec: &Vec<FPDecimal>) -> FPDecimal {
    vec.iter().fold(FPDecimal::zero(), |acc, &el| acc + el)
}

pub fn dot(vec: &Vec<FPDecimal>, other: &Vec<FPDecimal>) -> FPDecimal {
    let mut sum = FPDecimal::zero();
    for i in 0..vec.len() {
        sum = sum + vec[i] * other[i]
    }
    sum
}

pub fn mul(vec: &Vec<FPDecimal>, other: &Vec<FPDecimal>) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 * i2).collect()
}

pub fn mul_const(vec: &Vec<FPDecimal>, other: FPDecimal) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i * other).collect()
}

pub fn div_const(vec: &Vec<FPDecimal>, other: FPDecimal) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i / other).collect()
}

pub fn add(vec: &Vec<FPDecimal>, other: &Vec<FPDecimal>) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 + i2).collect()
}

pub fn sub(vec: &Vec<FPDecimal>, other: &Vec<FPDecimal>) -> Vec<FPDecimal> {
    vec.iter().zip(other).map(|(&i1, &i2)| i1 - i2).collect()
}

pub fn abs(vec: &Vec<FPDecimal>) -> Vec<FPDecimal> {
    vec.iter().map(|&i| i.abs()).collect()
}