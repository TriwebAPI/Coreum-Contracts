use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct TokenizedAsset {
    pub owner: Addr,
    pub total_supply: Uint128,
    pub remaining_supply: Uint128,
    pub price: Uint128,
    pub uri: String,
    pub asset_type: AssetType,
}

#[cw_serde]
pub enum AssetType {
    RealWorldAsset,
    IntellectualProperty,
    BondOrSecurity,
}

pub const ASSETS: Map<u64, TokenizedAsset> = Map::new("assets");
pub const NEXT_TOKEN_ID: Item<u64> = Item::new("next_token_id");
pub const FRACTIONAL_BALANCES: Map<(Addr, u64), Uint128> = Map::new("fractional_balances");