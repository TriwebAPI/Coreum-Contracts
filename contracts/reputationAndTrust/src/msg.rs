use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The `InstantiateMsg` struct contains the parameters needed to initialize the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// The symbol of the token to be issued.
    pub symbol: String,
    /// The subunit name of the token.
    pub subunit: String,
    /// The precision (number of decimal places) of the token.
    pub precision: u32,
    /// The initial amount of the token to be issued.
    pub initial_amount: Uint128,
}

/// The `ExecuteMsg` enum defines the different execute messages that can be sent to the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Updates the reputation of a specified user. Only callable by the contract owner.
    UpdateReputation { 
        /// The address of the user whose reputation is to be updated.
        user: String, 
        /// The new reputation value for the user.
        reputation: u64 
    },
    /// Resets the reputation of a specified user to zero. Only callable by the contract owner.
    ResetReputation { 
        /// The address of the user whose reputation is to be reset.
        user: String 
    },
    /// Transfers a specified amount of tokens to a recipient.
    Transfer { 
        /// The address of the recipient to whom the tokens will be transferred.
        recipient: String, 
        /// The amount of tokens to be transferred.
        amount: Uint128 
    },
}

/// The `QueryMsg` enum defines the different query messages that can be sent to the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Queries and returns the details of the token issued by the contract.
    Token {},
    /// Queries and returns the reputation of a specified user.
    GetReputation { 
        /// The address of the user whose reputation is to be queried.
        user: String 
    },
    /// Queries and returns the token balance of a specified user.
    GetBalance { 
        /// The address of the user whose balance is to be queried.
        user: String 
    },
}