use cosmwasm_std::StdError;
use thiserror::Error;

/// ## Description
/// This enum describes factory contract errors.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Generic(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No cluster registration process in progress")]
    NoRegistrationInProgress {},
}
