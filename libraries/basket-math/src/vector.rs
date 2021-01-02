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
