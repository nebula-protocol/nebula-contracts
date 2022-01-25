use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    Generic(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0} too short")]
    ValueTooShort(String),

    #[error("{0} too long")]
    ValueTooLong(String),

    #[error("{0} must be {1} to {2}")]
    ValueOutOfRange(String, Uint128, Uint128),

    #[error("{0} has not expired")]
    ValueHasNotExpired(String),

    #[error("Poll does not exist")]
    PollNotExists {},

    #[error("Poll is not in progress")]
    PollNotInProgress {},

    #[error("Nothing staked")]
    NothingStaked {},

    #[error("Nothing to withdraw")]
    NothingToWithdraw {},
}
