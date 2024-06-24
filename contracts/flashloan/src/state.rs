use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// State struct to hold contract state data
#[cw_serde]
pub struct State {
    /// Owner of the contract
    pub owner: Addr,
    /// Address of the lending pool
    pub lending_pool: Addr,
}

/// Constant to store the state data in the contract's storage
pub const STATE: Item<State> = Item::new("state");