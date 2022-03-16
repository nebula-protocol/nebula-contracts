use cosmwasm_std::StdError;
use thiserror::Error;

/// ## Description
/// This enum describes incentives custody contract errors.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
}
