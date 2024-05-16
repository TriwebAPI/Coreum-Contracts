use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Contract Currently Paused")]
    PausedContract {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}