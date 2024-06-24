use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct State {
    pub owner: Addr,
    pub marketplace: Addr,
}

pub const STATE: Item<State> = Item::new("state");

#[cw_serde]
pub struct NFT {
    pub id: String,
    pub owner: Addr,
    pub metadata: String,
    pub royalties: Option<u64>,
}

#[cw_serde]
pub struct SaleInfo {
    pub price: Uint128,
    pub royalty: Option<u64>,
}

pub const SALES: Map<String, SaleInfo> = Map::new("sales");
pub const NFTS: Map<String, NFT> = Map::new("nfts");
pub const EDITIONS: Map<String, u32> = Map::new("editions");
pub const RENTALS: Map<String, (Addr, u64)> = Map::new("rentals");