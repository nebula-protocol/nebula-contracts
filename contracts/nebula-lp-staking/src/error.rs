use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

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

    #[error("Missing {0}")]
    Missing(String),

    #[error("Unauthorized")]
    Unauthorized {},
}
