use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Addr};

use crate::state::NFT;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub marketplace: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateNFT { id: String, metadata: String, royalties: Option<u64> },
    ListForSale { id: String, price: Uint128 },
    BuyNFT { id: String },
    RentNFT { id: String, duration: u64 },
    ReturnNFT { id: String },
    MintEdition { id: String, edition: u32 },
    UpdateNFT { id: String, new_metadata: String },
    WithdrawFunds {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(NFT)]
    GetNFT { id: String },
    #[returns(Uint128)]
    GetNFTPrice { id: String },
    #[returns((Addr, u64))]
    GetRentalInfo { id: String },
}