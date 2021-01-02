use crate::FPDecimal;
use crate::{dot, sum};

/// Calculate error for basket's current inventory and its target weight allocation.
pub fn compute_err(
    inv: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
    target: &Vec<u32>, // not normalized
) -> Vec<FPDecimal> {
    // w: Vec<FPDecimal> = normalized target vector (target / sum(target))
    let target_sum = target
        .iter()
        .fold(FPDecimal::zero(), |acc, &el| acc + FPDecimal::from(el));
    let w: Vec<FPDecimal> = target
        .iter()
        .map(|&x| FPDecimal::from(x) / target_sum)
        .collect();

    // u: Vec<FPDecimal> = (w.elementMul(p))/w.dot(p)
    let mut u = Vec::<FPDecimal>::new();
    let denom = dot(&w, &p);
    for i in 0..inv.len() {
        u.push(w[i] * p[i] / denom)
    }

    // err: Vec<Decimal> = inv.dot(p) * u - inv.elementMul(p)
    let mut err = Vec::<FPDecimal>::new();
    let prod = dot(inv, &p);
    for i in 0..inv.len() {
        err.push(prod * u[i] - inv[i] * p[i]);
    }

    err
}

pub fn compute_diff(
    inv: &Vec<FPDecimal>,
    c: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
    target: &Vec<u32>,
) -> Vec<FPDecimal> {
    // abs(err(inv, p, target)) - abs(err(inv + delta, p, target))
    let mut inv_p = Vec::<FPDecimal>::new();
    for i in 0..inv.len() {
        inv_p.push(inv[i] + c[i])
    }

    let err1 = compute_err(inv, &p, &target);
    let err2 = compute_err(&inv_p, &p, &target);

    let mut diff = Vec::<FPDecimal>::new();
    for i in 0..err1.len() {
        diff.push(err1[i].abs() - err2[i].abs())
    }

    diff
}

/// Calculates score penalty for inventory basket given a delta. The delta is
/// a vector with the same dimensions as inventory, and can be negative.
/// Returns: (X - score)
pub fn compute_score(
    inv: &Vec<FPDecimal>,
    c: &Vec<FPDecimal>,
    target: &Vec<u32>,
    p: &Vec<FPDecimal>,
) -> FPDecimal {
    // compute X (score)
    // X: Decimal = sum(diff) / dot(delta, prices)
    // diff: Vec<Decimal> = |err(inventory + delta, prices, target)| - |err(inventory, weight, target)|
    let diff = compute_diff(inv, c, p, target);
    let score: FPDecimal = sum(&diff) / dot(c, &p);

    return score;
}

/// Given a score and penalty parameters for the penalty curve, determine the
/// resultant penalty value.
///
/// penalty(score) = if score <= 0: 1 - a_neg * tanh(score / s_neg)
///                  if score >  0: 1 - a_pos * tanh(score / s_pos)
pub fn compute_penalty(
    score: FPDecimal, // range: [-1, 1]
    a_pos: FPDecimal,
    s_pos: FPDecimal,
    a_neg: FPDecimal,
    s_neg: FPDecimal,
) -> FPDecimal {
    if score.0 <= 0 {
        FPDecimal::one() - a_neg * (score / s_neg).tanh()
    } else {
        FPDecimal::one() - a_pos * (score / s_pos).tanh()
    }
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn test_compute_penalty() {
    //     let inv = vec![FPDecimal::from(5).div(3), FPDecimal::from(3)];
    //     let c = vec![FPDecimal::from(7).div(2), FPDecimal::from(3).div(2)];
    //     let target = vec![1, 2];
    //     let p = vec![FPDecimal::from(1), FPDecimal::from(1).div(2)];

    //     let penalty_params = PenaltyParams {
    //         alpha_plus: FPDecimal::ONE,            // 1
    //         alpha_minus: FPDecimal::ONE / 2 / 100, // 0.005
    //         sigma_plus: FPDecimal::ONE,            // 1
    //         sigma_minus: FPDecimal::ONE / 2,       // 0.5
    //     };

    //     let score = compute_score(&inv, &c, &target, &p);
    //     let penalty = compute_penalty(&score, &penalty_params);
    // }
}
