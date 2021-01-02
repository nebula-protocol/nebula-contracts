use basket_math;

fn main() {
    let penalty_params = PenaltyParams {
        alpha_plus: Fixed::ONE,            // 1
        alpha_minus: Fixed::ONE / 2 / 100, // 0.005
        sigma_plus: Fixed::ONE,            // 1
        sigma_minus: Fixed::ONE / 2,       // 0.5
    };
}
