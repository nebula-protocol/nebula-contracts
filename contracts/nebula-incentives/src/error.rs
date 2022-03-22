use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

/// ## Description
/// This enum describes incentives contract errors.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    Generic(String),

    #[error("Invalid {0}")]
    Invalid(String),

    #[error("Unauthorized")]
    Unauthorized {},
}
