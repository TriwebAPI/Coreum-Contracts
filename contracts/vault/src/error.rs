use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug,PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient balance")]
    InsufficientBalance {},
    
    #[error("Address not whitelisted")]
    NotWhitelisted {},

    #[error("Overflow error")]
    Overflow {},

    #[error("Divide by zero error")]
    DivideByZero {},

    #[error("Insufficient funds")]
    InsufficientFunds {},
    

    #[error("To Do Error")]
    ToDo {},
}