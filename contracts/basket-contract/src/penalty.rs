use crate::util::vec_to_string;
use basket_math::{dot, sum, FPDecimal};

/// Calculate error for basket's current inventory and its target weight allocation.
pub fn compute_err(
    inv: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
    target: &Vec<u32>, // not normalized
) -> Vec<FPDecimal> {
    println!(
        "compute_err(inv: {}, p: {}, target: {})",
        vec_to_string(&inv),
        vec_to_string(&p),
        vec_to_string(&target)
    );
    // w: Vec<FPDecimal> = normalized target vector (target / sum(target))
    let target_sum = target
        .iter()
        .fold(FPDecimal::zero(), |acc, &el| acc + FPDecimal::from(el));
    let w: Vec<FPDecimal> = target
        .iter()
        .map(|&x| FPDecimal::from(x) / target_sum)
        .collect();
    println!("\tw = normalize(target) = target / sum(target)");
    println!("\t  = {}", vec_to_string(&w));

    // u: Vec<FPDecimal> = (w.elementMul(p))/w.dot(p)
    let mut u = Vec::<FPDecimal>::new();
    let denom = dot(&w, &p);
    for i in 0..inv.len() {
        u.push(w[i] * p[i] / denom)
    }
    println!("\tu = w.mul(p) / w.dot(p)");
    println!("\t  = {}", vec_to_string(&u));

    // e: Vec<Decimal> = inv.dot(p) * u - inv.elementMul(p)
    let mut e = Vec::<FPDecimal>::new();
    let mut inv_mul_p = Vec::<FPDecimal>::new(); // for debug
    let prod = dot(inv, &p);
    for i in 0..inv.len() {
        e.push(prod * u[i] - inv[i] * p[i]);
        inv_mul_p.push(inv[i] * p[i]);
    }
    println!("\tinv.dot(p) = {}", prod);
    println!("\tinv.mul(p) = {}", vec_to_string(&inv_mul_p));
    println!("\te = (inv.dot(p) * u) - inv.mul(p)");
    println!("\t  = {}", vec_to_string(&e));
    println!("return compute_err -> {}", vec_to_string(&e));
    e
}

pub fn compute_diff(
    inv: &Vec<FPDecimal>,
    c: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
    target: &Vec<u32>,
) -> Vec<FPDecimal> {
    println!(
        "compute_diff(inv: {}, c: {}, p:{}, target: {})",
        vec_to_string(&inv),
        vec_to_string(&c),
        vec_to_string(&p),
        vec_to_string(&target)
    );

    // abs(err(inv + c, p, target)) - abs(err(inv, p, target))
    let mut inv_p = Vec::<FPDecimal>::new();
    for i in 0..inv.len() {
        inv_p.push(inv[i] + c[i])
    }
    println!("\tinv + c = {}", vec_to_string(&inv_p));
    let err = compute_err(inv, &p, &target);
    let err_p = compute_err(&inv_p, &p, &target);
    println!("\terr(inv + c, p, target) = {}", vec_to_string(&err_p));
    println!("\terr(inv, p, target) = {}", vec_to_string(&err));

    let mut diff = Vec::<FPDecimal>::new();
    for i in 0..err.len() {
        diff.push(err_p[i].abs() - err[i].abs())
    }

    println!("\tdiff = |err(inv + c, p, target)| - |err(inv, p, target)|");
    println!("\t     = {}", vec_to_string(&diff));
    println!("return compute_diff -> {}", vec_to_string(&diff));
    diff
}

/// Calculates score penalty for inventory basket given a delta. The delta is
/// a vector with the same dimensions as inventory, and can be negative.
/// Returns: (X - score)
pub fn compute_score(
    inv: &Vec<FPDecimal>,
    c: &Vec<FPDecimal>,
    p: &Vec<FPDecimal>,
    target: &Vec<u32>,
) -> FPDecimal {
    println!(
        "compute_score(inv: {}, c: {}, p: {}, target: {})",
        vec_to_string(&inv),
        vec_to_string(&c),
        vec_to_string(&p),
        vec_to_string(&target)
    );
    // compute X (score)
    // X: Decimal = sum(diff) / dot(delta, prices)
    // diff: Vec<Decimal> = |err(inventory + delta, prices, target)| - |err(inventory, weight, target)|
    let diff = compute_diff(inv, c, p, target);
    let score: FPDecimal = sum(&diff) / dot(c, &p);
    println!("\tsum(diff) = {}", sum(&diff));
    println!("\tc.dot(p) = {}", dot(c, &p));
    println!("\tscore = sum(diff) / c.dot(p)");
    println!("\t      = {}", score);
    println!("return: compute_score -> {}", score);
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
