use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The `State` struct holds global state information for the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// The owner of the contract.
    pub owner: Addr,
    /// The denomination of the token issued by the contract.
    pub denom: String,
}

/// `STATE` is an `Item` storage entry that holds a single instance of the `State` struct.
pub const STATE: Item<State> = Item::new("state");

/// The `UserReputation` struct holds the reputation value for a specific user.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserReputation {
    /// The reputation value of the user.
    pub reputation: u64,
}

/// `REPUTATIONS` is a `Map` storage entry that maps a user's address to their `UserReputation`.
pub const REPUTATIONS: Map<&Addr, UserReputation> = Map::new("reputations");

/// `BALANCES` is a `Map` storage entry that maps a user's address to their token balance.
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balances");