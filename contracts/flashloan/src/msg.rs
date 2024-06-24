use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Uint128};

use crate::state::State;

/// Message used to instantiate the contract, setting the owner and lending pool addresses.
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub lending_pool: String,
}

/// Enumeration of messages that can be executed by the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Request a flash loan with specified token, amount, and collateral.
    RequestFlashLoan { token: String, amount: Uint128, collateral: Uint128 },
    /// Execute the flash loan operation, repaying the loan with a premium.
    ExecuteOperation { token: String, amount: Uint128, premium: Uint128 },
    /// Withdraw the specified token's balance (only callable by the owner).
    Withdraw { token: String },
}

/// Enumeration of messages that can be queried from the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the balance of the specified token.
    #[returns(Uint128)]
    GetBalance { token: String },
    /// Query the current state of the loan.
    #[returns(State)]
    LoanInfo {},
}

/// Structure representing a request for a flash loan.
#[cw_serde]
pub struct RequestFlashLoan {
    pub recipient: String,
    pub token: String,
    pub amount: Uint128,
}

/// Structure representing the repayment of a flash loan.
#[cw_serde]
pub struct RepayFlashLoan {
    pub sender: String,
    pub token: String,
    pub amount: Uint128,
}

/// Enumeration of custom messages used by the contract.
#[cw_serde]
pub enum CustomMsg {
    /// Custom message to request a flash loan.
    RequestFlashLoan(RequestFlashLoan),
    /// Custom message to repay a flash loan.
    RepayFlashLoan(RepayFlashLoan),
}

/// Implement conversion from CustomMsg to CosmosMsg for use in the contract.
impl From<CustomMsg> for CosmosMsg<CustomMsg> {
    fn from(msg: CustomMsg) -> Self {
        CosmosMsg::Custom(msg)
    }
}