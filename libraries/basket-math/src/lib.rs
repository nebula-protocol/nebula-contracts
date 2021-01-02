pub mod fp_decimal;
pub mod penalty;
pub mod vector;

pub use fp_decimal::*;
pub use vector::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
