use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Storage, Uint128};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Proposal {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub votes_for: Uint128,
    pub votes_against: Uint128,
    pub executed: bool,
    pub amount: Uint128,
    pub recipient: Addr,
    pub voting_end: u64, // UNIX timestamp
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Member {
    pub address: Addr,
    pub weight: Uint128
}


pub const STATE: Item<()> = Item::new("state");
pub const PROPOSALS: Map<&str, Proposal> = Map::new("proposals");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const MEMBERS: Map<&str, Member> = Map::new("members");