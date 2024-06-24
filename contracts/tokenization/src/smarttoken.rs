// Contents of smarttoken.rs

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};
use coreum_wasm_sdk::assetft;
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries};

const CONTRACT_NAME: &str = "smart-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InstantiateMsg {
    pub owner: String,
    pub symbol: String,
    pub subunit: String,
    pub precision: u32,
    pub initial_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ExecuteMsg {
    Mint { to: String, amount: Uint128 },
    Transfer { to: String, amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum QueryMsg {
    Balance { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TokenInfo {
    pub owner: Addr,
    pub total_supply: Uint128,
    pub denom: String,
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");
pub const BALANCES: Map<Addr, Uint128> = Map::new("balances");

#[entry_point]
pub fn instantiate(
    deps: DepsMut<CoreumQueries>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<CoreumMsg>> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    let denom = format!("{}-{}", msg.subunit, env.contract.address).to_lowercase();

    let token_info = TokenInfo {
        owner: owner.clone(),
        total_supply: msg.initial_amount,
        denom: denom.clone(),
    };
    TOKEN_INFO.save(deps.storage, &token_info)?;

    let issue_msg = CoreumMsg::AssetFT(assetft::Msg::Issue {
        symbol: msg.symbol,
        subunit: msg.subunit.clone(),
        precision: msg.precision,
        initial_amount: msg.initial_amount,
        description: None,
        features: Some(vec![0]), // 0 - minting
        burn_rate: Some("0".into()),
        send_commission_rate: Some("0.1".into()), // 10% commission for sending
    });

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner.to_string())
        .add_attribute("denom", denom)
        .add_message(issue_msg))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<CoreumQueries>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<CoreumMsg>> {
    match msg {
        ExecuteMsg::Mint { to, amount } => execute_mint(deps, info, to, amount),
        ExecuteMsg::Transfer { to, amount } => execute_transfer(deps, info, to, amount),
    }
}

fn execute_mint(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> StdResult<Response<CoreumMsg>> {
    let token_info = TOKEN_INFO.load(deps.storage)?;
    if info.sender != token_info.owner {
        return Err(StdError::generic_err("Only the owner can mint tokens"));
    }

    let to_addr = deps.api.addr_validate(&to)?;
    let balance = BALANCES.may_load(deps.storage, to_addr.clone())?.unwrap_or_default();
    BALANCES.save(deps.storage, to_addr.clone(), &(balance + amount))?;

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("to", to_addr.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn execute_transfer(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> StdResult<Response<CoreumMsg>> {
    let sender_addr = info.sender.clone();
    let to_addr = deps.api.addr_validate(&to)?;

    let sender_balance = BALANCES.load(deps.storage, sender_addr.clone())?;
    if sender_balance < amount {
        return Err(StdError::generic_err("Insufficient balance"));
    }

    BALANCES.save(deps.storage, sender_addr.clone(), &(sender_balance - amount))?;

    let recipient_balance = BALANCES.may_load(deps.storage, to_addr.clone())?.unwrap_or_default();
    BALANCES.save(deps.storage, to_addr.clone(), &(recipient_balance + amount))?;

    Ok(Response::new()
        .add_attribute("method", "transfer")
        .add_attribute("from", sender_addr.to_string())
        .add_attribute("to", to_addr.to_string())
        .add_attribute("amount", amount.to_string()))
}

#[entry_point]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
    }
}

fn query_balance(deps: Deps<CoreumQueries>, address: String) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&address)?;
    let balance = BALANCES.may_load(deps.storage, addr)?.unwrap_or_default();
    Ok(balance)
}