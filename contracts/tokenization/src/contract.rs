use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, AssetType as MsgAssetType};
use crate::state::{TokenizedAsset, ASSETS, FRACTIONAL_BALANCES, NEXT_TOKEN_ID, AssetType as StateAssetType};
use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg
};
use cw2::set_contract_version;
use crate::smarttoken::{BALANCES, TOKEN_INFO};
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries};

const CONTRACT_NAME: &str = "asset-tokenization";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    NEXT_TOKEN_ID.save(deps.storage, &1)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("method", "instantiate").add_attribute("owner", owner.to_string()))
}

#[entry_point]
pub fn execute(
    deps:  DepsMut<CoreumQueries>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    match msg {
        ExecuteMsg::CreateAsset { total_supply, price, uri, asset_type } => create_asset(deps, info, total_supply, price, uri, asset_type),
        ExecuteMsg::TransferOwnership { token_id, to, amount } => transfer_ownership(deps, info, token_id, to, amount),
        ExecuteMsg::MintSmartToken { to, amount } => execute_mint_smart_token(deps, info, to, amount),
        ExecuteMsg::TransferSmartToken { to, amount } => execute_transfer_smart_token(deps, info, to, amount),
    }
}

fn create_asset(
    deps:DepsMut<CoreumQueries>,
    info: MessageInfo,
    total_supply: Uint128,
    price: Uint128,
    uri: String,
    asset_type: MsgAssetType,
) -> Result<Response<CoreumMsg>, ContractError> {
    let owner = info.sender.clone();
    let token_id = NEXT_TOKEN_ID.load(deps.storage)?;

    let asset_type = match asset_type {
        MsgAssetType::RealWorldAsset => StateAssetType::RealWorldAsset,
    };

    let asset = TokenizedAsset {
        owner: owner.clone(),
        total_supply,
        remaining_supply: total_supply,
        price,
        uri,
        asset_type,
    };

    ASSETS.save(deps.storage, token_id, &asset)?;
    NEXT_TOKEN_ID.save(deps.storage, &(token_id + 1))?;

    Ok(Response::new().add_attribute("method", "create_asset").add_attribute("token_id", token_id.to_string()).add_attribute("owner", owner.to_string()))
}

fn transfer_ownership(
    deps:DepsMut<CoreumQueries>,
    info: MessageInfo,
    token_id: u64,
    to: String,
    amount: Uint128,
) -> Result<Response<CoreumMsg>, ContractError> {
    let mut asset = ASSETS.load(deps.storage, token_id)?;

    if info.sender != asset.owner {
        return Err(ContractError::Unauthorized {});
    }

    if amount > asset.remaining_supply {
        return Err(ContractError::Std(StdError::generic_err("Invalid amount to transfer")));
    }

    asset.remaining_supply = asset.remaining_supply.checked_sub(amount).map_err(|e| ContractError::Std(StdError::generic_err(format!("Overflow error: {}", e))))?;
    ASSETS.save(deps.storage, token_id, &asset)?;

    let to_addr = deps.api.addr_validate(&to)?;
    let balance = FRACTIONAL_BALANCES.may_load(deps.storage, (to_addr.clone(), token_id))?.unwrap_or_default();
    FRACTIONAL_BALANCES.save(deps.storage, (to_addr.clone(), token_id), &(balance + amount))?;

    Ok(Response::new().add_attribute("method", "transfer_ownership").add_attribute("token_id", token_id.to_string()).add_attribute("from", info.sender.to_string()).add_attribute("to", to_addr.to_string()).add_attribute("amount", amount.to_string()))
}

/// Mint new smart tokens
fn execute_mint_smart_token(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> Result<Response<CoreumMsg>, ContractError> {
    let token_info = TOKEN_INFO.load(deps.storage)?;

    // Ensure the sender is the owner of the token
    if info.sender != token_info.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Update the recipient's balance
    let to_addr = deps.api.addr_validate(&to)?;
    let balance = BALANCES.may_load(deps.storage, to_addr.clone())?.unwrap_or_default();
    BALANCES.save(deps.storage, to_addr.clone(), &(balance + amount))?;

    Ok(Response::new()
        .add_attribute("method", "mint_smart_token")
        .add_attribute("to", to_addr.to_string())
        .add_attribute("amount", amount.to_string()))
}

/// Transfer smart tokens
fn execute_transfer_smart_token(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> Result<Response<CoreumMsg>, ContractError> {
    let sender_addr = info.sender.clone();
    let to_addr = deps.api.addr_validate(&to)?;

    // Ensure the sender has enough balance
    let sender_balance = BALANCES.load(deps.storage, sender_addr.clone())?;
    if sender_balance < amount {
        return Err(ContractError::Unauthorized {});
    }

    // Update the sender's and recipient's balances
    BALANCES.save(deps.storage, sender_addr.clone(), &(sender_balance - amount))?;
    let recipient_balance = BALANCES.may_load(deps.storage, to_addr.clone())?.unwrap_or_default();
    BALANCES.save(deps.storage, to_addr.clone(), &(recipient_balance + amount))?;

    Ok(Response::new()
        .add_attribute("method", "transfer_smart_token")
        .add_attribute("from", sender_addr.to_string())
        .add_attribute("to", to_addr.to_string())
        .add_attribute("amount", amount.to_string()))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::FractionalOwnership { token_id, owner } => to_binary(&query_fractional_ownership(deps, token_id, owner)?),
        QueryMsg::TokenURI { token_id } => to_binary(&query_token_uri(deps, token_id)?),
    }
}

fn query_fractional_ownership(deps: Deps, token_id: u64, owner: String) -> StdResult<Uint128> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let balance = FRACTIONAL_BALANCES.may_load(deps.storage, (owner_addr, token_id))?.unwrap_or_default();
    Ok(balance)
}

fn query_token_uri(deps: Deps, token_id: u64) -> StdResult<String> {
    let asset = ASSETS.load(deps.storage, token_id)?;
    Ok(asset.uri)
}
