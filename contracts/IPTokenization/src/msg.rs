use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub symbol: String,
    pub subunit: String,
    pub precision: u8,
    pub initial_amount: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateAsset { total_supply: Uint128, price: Uint128, uri: String, asset_type: AssetType },
    MintSmartToken { to: String, amount: Uint128 },
    TransferSmartToken { to: String, amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

    #[returns(String)]
    TokenURI { token_id: u64 }
}

#[cw_serde]
pub enum AssetType {
    IntellectualProperty
}