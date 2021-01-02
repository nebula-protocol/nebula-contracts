use basket_math::FPDecimal;
use std::str::FromStr;

fn main() {
    println!(
        "{}",
        FPDecimal::from_str("-123.12301293012931").unwrap()
            * FPDecimal::from_str("-1232329.294230499").unwrap()
            / FPDecimal::from_str("-28281.2102392039111").unwrap()
            * FPDecimal::from_str("213.123123019230").unwrap()
    );
}
