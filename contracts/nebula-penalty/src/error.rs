use cosmwasm_std::StdError;
use thiserror::Error;

/// ## Description
/// This enum describes penalty contract errors.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Generic(String),

    #[error("Unauthorized")]
    Unauthorized {},
}
