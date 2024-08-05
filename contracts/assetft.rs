use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, QueryRequest,
    Response, StdError, StdResult, Storage, Uint128, WasmQuery,
};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::pagination::{PageRequest, PageResponse};

pub const MINTING: u32 = 0;
pub const BURNING: u32 = 1;
pub const FREEZING: u32 = 2;
pub const WHITELISTING: u32 = 3;
pub const IBC: u32 = 4;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Params {
    pub issue_fee: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ParamsResponse {
    pub params: Params,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Token {
    pub denom: String,
    pub issuer: String,
    pub symbol: String,
    pub subunit: String,
    pub precision: u32,
    pub description: Option<String>,
    pub features: Option<Vec<u32>>,
    pub burn_rate: String,
    pub send_commission_rate: String,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokensResponse {
    pub pagination: PageResponse,
    pub tokens: Vec<Token>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenResponse {
    pub token: Token,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BalanceResponse {
    pub balance: Uint128,
    pub whitelisted: bool,
    pub frozen: bool,
    pub locked: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FrozenBalancesResponse {
    pub pagination: PageResponse,
    pub balances: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FrozenBalanceResponse {
    pub balance: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WhitelistedBalancesResponse {
    pub pagination: PageResponse,
    pub balances: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WhitelistedBalanceResponse {
    pub balance: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Msg {
    Issue {
        symbol: String,
        subunit: String,
        precision: u32,
        initial_amount: Uint128,
        description: Option<String>,
        features: Option<Vec<u32>>,
        burn_rate: Option<String>,
        send_commission_rate: Option<String>,
    },
    Mint {
        denom: String,
        amount: Uint128,
    },
    Burn {
        denom: String,
        amount: Uint128,
    },
    Freeze {
        account: String,
        denom: String,
    },
    Unfreeze {
        account: String,
        denom: String,
    },
    GloballyFreeze {
        denom: String,
    },
    GloballyUnfreeze {
        denom: String,
    },
    SetWhitelistedLimit {
        account: String,
        denom: String,
    },
    UpgradeTokenV1 {
        denom: String,
        ibc_enabled: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
pub enum Query {
    #[returns(ParamsResponse)]
    Params {},

    #[returns(TokensResponse)]
    Tokens {
        pagination: Option<PageRequest>,
        issuer: String,
    },

    #[returns(TokenResponse)]
    Token { denom: String },

    #[returns(BalanceResponse)]
    Balance { account: String, denom: String },

    #[returns(FrozenBalancesResponse)]
    FrozenBalances {
        pagination: Option<PageRequest>,
        account: String,
    },

    #[returns(FrozenBalanceResponse)]
    FrozenBalance { account: String, denom: String },

    #[returns(WhitelistedBalancesResponse)]
    WhitelistedBalances {
        pagination: Option<PageRequest>,
        account: String,
    },

    #[returns(WhitelistedBalanceResponse)]
    WhitelistedBalance { account: String, denom: String },
}

// Custom error type for transfer restriction errors
#[derive(Debug)]
pub enum ContractError {
    TransferRestricted { reason: String },
    Unauthorized { reason: String },
    InvalidRequest { reason: String },
    TokenNotFound,
}

// Storage keys
const BALANCES: Map<(&str, &str), Uint128> = Map::new("balances");
const FROZEN_ACCOUNTS: Map<(&str, &str), bool> = Map::new("frozen_accounts");
const WHITELISTED_ACCOUNTS: Map<(&str, &str), bool> = Map::new("whitelisted_accounts");
const TOKENS: Map<&str, Token> = Map::new("tokens");
const GLOBAL_FREEZE: Item<HashMap<String, bool>> = Item::new("global_freeze");

// Implementing restrictions checks

// Check if an account is frozen
pub fn is_frozen(store: &dyn Storage, account: &str, denom: &str) -> bool {
    FROZEN_ACCOUNTS
        .may_load(store, (account, denom))
        .unwrap_or_default()
        .unwrap_or(false)
}

// Check if an account is whitelisted
pub fn is_whitelisted(store: &dyn Storage, account: &str, denom: &str) -> bool {
    WHITELISTED_ACCOUNTS
        .may_load(store, (account, denom))
        .unwrap_or_default()
        .unwrap_or(false)
}

// Check if a global freeze is in effect for a token
pub fn is_globally_frozen(store: &dyn Storage, denom: &str) -> bool {
    GLOBAL_FREEZE
        .may_load(store)
        .unwrap_or_default()
        .map(|map| *map.get(denom).unwrap_or(&false))
        .unwrap_or(false)
}

// Check if a transfer is allowed
pub fn is_transfer_allowed(
    store: &dyn Storage,
    sender: &str,
    recipient: &str,
    denom: &str,
) -> Result<(), ContractError> {
    if is_frozen(store, sender, denom) {
        return Err(ContractError::TransferRestricted {
            reason: "Sender account is frozen".to_string(),
        });
    }

    if is_frozen(store, recipient, denom) {
        return Err(ContractError::TransferRestricted {
            reason: "Recipient account is frozen".to_string(),
        });
    }

    if !is_whitelisted(store, recipient, denom) {
        return Err(ContractError::TransferRestricted {
            reason: "Recipient is not whitelisted".to_string(),
        });
    }

    if is_globally_frozen(store, denom) {
        return Err(ContractError::TransferRestricted {
            reason: "Token is globally frozen".to_string(),
        });
    }

    Ok(())
}

// Function to provide error message for a given restriction
pub fn restriction_message(restriction: ContractError) -> String {
    match restriction {
        ContractError::TransferRestricted { reason } => reason,
        _ => "No restrictions".to_string(),
    }
}

// Implementing the Msg handlers with restrictions

// Minting tokens with restrictions
pub fn mint(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    amount: Uint128,
) -> Result<Response, StdError> {
    // For simplicity, assume minting is unrestricted for this example
    // In a real-world scenario, you might check if the sender is authorized to mint

    // Update the state to reflect the minted amount
    let key = (info.sender.as_str(), denom.as_str());
    let mut balance = BALANCES.may_load(deps.storage, key)?.unwrap_or(Uint128::zero());
    balance += amount;
    BALANCES.save(deps.storage, key, &balance)?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("amount", amount.to_string()))
}

// Burning tokens
pub fn burn(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    amount: Uint128,
) -> Result<Response, StdError> {
    let key = (info.sender.as_str(), denom.as_str());
    let mut balance = BALANCES.load(deps.storage, key)?;

    if balance < amount {
        return Err(StdError::generic_err("Insufficient balance to burn"));
    }

    balance -= amount;
    BALANCES.save(deps.storage, key, &balance)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("amount", amount.to_string()))
}

// Freezing an account's balance
pub fn freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account: String,
    denom: String,
) -> Result<Response, StdError> {
    // Only allow the contract owner or issuer to freeze
    // Assuming `info.sender` is checked against an admin list or issuer

    FROZEN_ACCOUNTS.save(deps.storage, (&account, &denom), &true)?;

    Ok(Response::new()
        .add_attribute("action", "freeze")
        .add_attribute("account", account)
        .add_attribute("denom", denom))
}

// Unfreezing an account's balance
pub fn unfreeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account: String,
    denom: String,
) -> Result<Response, StdError> {
    // Only allow the contract owner or issuer to unfreeze
    // Assuming `info.sender` is checked against an admin list or issuer

    FROZEN_ACCOUNTS.save(deps.storage, (&account, &denom), &false)?;

    Ok(Response::new()
        .add_attribute("action", "unfreeze")
        .add_attribute("account", account)
        .add_attribute("denom", denom))
}

// Globally freezing a token
pub fn globally_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, StdError> {
    // Only allow the contract owner or issuer to globally freeze
    // Assuming `info.sender` is checked against an admin list or issuer

    let mut global_freeze = GLOBAL_FREEZE.load(deps.storage)?;
    global_freeze.insert(denom.clone(), true);
    GLOBAL_FREEZE.save(deps.storage, &global_freeze)?;

    Ok(Response::new()
        .add_attribute("action", "globally_freeze")
        .add_attribute("denom", denom))
}

// Globally unfreezing a token
pub fn globally_unfreeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, StdError> {
    // Only allow the contract owner or issuer to globally unfreeze
    // Assuming `info.sender` is checked against an admin list or issuer

    let mut global_freeze = GLOBAL_FREEZE.load(deps.storage)?;
    global_freeze.insert(denom.clone(), false);
    GLOBAL_FREEZE.save(deps.storage, &global_freeze)?;

    Ok(Response::new()
        .add_attribute("action", "globally_unfreeze")
        .add_attribute("denom", denom))
}

// Setting a whitelisted limit for an account
pub fn set_whitelisted_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account: String,
    denom: String,
) -> Result<Response, StdError> {
    // Only allow the contract owner or issuer to set whitelisted limits
    // Assuming `info.sender` is checked against an admin list or issuer

    WHITELISTED_ACCOUNTS.save(deps.storage, (&account, &denom), &true)?;

    Ok(Response::new()
        .add_attribute("action", "set_whitelisted_limit")
        .add_attribute("account", account)
        .add_attribute("denom", denom))
}

// Transferring tokens with restriction checks
pub fn transfer(
    deps: DepsMut,
    info: MessageInfo,
    recipient: String,
    coin: Coin,
) -> Result<Response, StdError> {
    let sender = info.sender.clone();
    let denom = &coin.denom;

    // Check for transfer restrictions
    match is_transfer_allowed(deps.storage, &sender.to_string(), &recipient, denom) {
        Ok(_) => {
            // Perform the transfer logic
            // Update balances in contract state
            let sender_key = (sender.as_str(), denom.as_str());
            let recipient_key = (recipient.as_str(), denom.as_str());

            let mut sender_balance = BALANCES.load(deps.storage, sender_key)?;
            if sender_balance < coin.amount {
                return Err(StdError::generic_err("Insufficient balance"));
            }

            sender_balance -= coin.amount;
            BALANCES.save(deps.storage, sender_key, &sender_balance)?;

            let mut recipient_balance = BALANCES
                .may_load(deps.storage, recipient_key)?
                .unwrap_or(Uint128::zero());
            recipient_balance += coin.amount;
            BALANCES.save(deps.storage, recipient_key, &recipient_balance)?;

            Ok(Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("from", sender)
                .add_attribute("to", recipient)
                .add_attribute("amount", coin.amount.to_string()))
        }
        Err(e) => Err(StdError::generic_err(restriction_message(e))),
    }
}

// Queries
pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let params = Params {
        issue_fee: Coin {
            denom: "example_coin".to_string(),
            amount: Uint128::new(1000),
        },
    };
    Ok(ParamsResponse { params })
}

pub fn query_tokens(deps: Deps, pagination: Option<PageRequest>, issuer: String) -> StdResult<TokensResponse> {
    // Query logic for tokens
    // Example: Get all tokens issued by the given issuer
    // Pagination and filtering logic would go here
    let tokens = vec![]; // Example placeholder
    let pagination = PageResponse {
        next_key: None,
        total: 0,
    };
    Ok(TokensResponse { pagination, tokens })
}

pub fn query_token(deps: Deps, denom: String) -> StdResult<TokenResponse> {
    // Query logic for a single token
    // Example: Get details for the token with the given denom
    let token = TOKENS.load(deps.storage, &denom)?;
    Ok(TokenResponse { token })
}

pub fn query_balance(deps: Deps, account: String, denom: String) -> StdResult<BalanceResponse> {
    // Query logic for a balance
    let balance = BALANCES
        .may_load(deps.storage, (&account, &denom))?
        .unwrap_or(Uint128::zero());
    let frozen = is_frozen(deps.storage, &account, &denom);
    let whitelisted = is_whitelisted(deps.storage, &account, &denom);
    let locked = Uint128::zero(); // Example placeholder for locked funds
    Ok(BalanceResponse {
        balance,
        whitelisted,
        frozen,
        locked,
    })
}

pub fn query_frozen_balances(
    deps: Deps,
    pagination: Option<PageRequest>,
    account: String,
) -> StdResult<FrozenBalancesResponse> {
    // Query logic for frozen balances
    let balances = vec![]; // Example placeholder
    let pagination = PageResponse {
        next_key: None,
        total: 0,
    };
    Ok(FrozenBalancesResponse { pagination, balances })
}

pub fn query_frozen_balance(deps: Deps, account: String, denom: String) -> StdResult<FrozenBalanceResponse> {
    // Query logic for a single frozen balance
    let balance = Coin {
        denom: denom.clone(),
        amount: Uint128::zero(), // Example placeholder
    };
    Ok(FrozenBalanceResponse { balance })
}

pub fn query_whitelisted_balances(
    deps: Deps,
    pagination: Option<PageRequest>,
    account: String,
) -> StdResult<WhitelistedBalancesResponse> {
    // Query logic for whitelisted balances
    let balances = vec![]; // Example placeholder
    let pagination = PageResponse {
        next_key: None,
        total: 0,
    };
    Ok(WhitelistedBalancesResponse { pagination, balances })
}

pub fn query_whitelisted_balance(
    deps: Deps,
    account: String,
    denom: String,
) -> StdResult<WhitelistedBalanceResponse> {
    // Query logic for a single whitelisted balance
    let balance = Coin {
        denom: denom.clone(),
        amount: Uint128::zero(), // Example placeholder
    };
    Ok(WhitelistedBalanceResponse { balance })
}