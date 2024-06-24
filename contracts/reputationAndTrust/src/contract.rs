use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, UserReputation, BALANCES, REPUTATIONS, STATE};
use coreum_wasm_sdk::assetft;
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "reputation-trust";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The instantiate function initializes the contract with the given parameters.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<CoreumQueries>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Set the contract version in the storage
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Prepare a message to issue a new fungible token (FT) using Coreum SDK
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

    // Define the token denom using the contract address
    let denom = format!("{}-{}", msg.subunit, env.contract.address).to_lowercase();
    let state = State {
        owner: info.sender.clone(),
        denom,
    };

    // Save the initial state in the storage
    STATE.save(deps.storage, &state)?;

    // Return a response with the necessary attributes and the issue message
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", state.owner.to_string())
        .add_attribute("denom", state.denom)
        .add_message(issue_msg))
}

/// The execute function handles different execute messages and performs the corresponding actions.
#[entry_point]
pub fn execute(
    deps: DepsMut<CoreumQueries>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    match msg {
        ExecuteMsg::UpdateReputation { user, reputation } => {
            update_reputation(deps, info, user, reputation)
        }
        ExecuteMsg::ResetReputation { user } => reset_reputation(deps, info, user),
        ExecuteMsg::Transfer { recipient, amount } => transfer(deps, info, recipient, amount),
    }
}

/// The update_reputation function allows the contract owner to update a user's reputation.
fn update_reputation(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    user: String,
    reputation: u64,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Load the current state from the storage
    let state = STATE.load(deps.storage)?;
    // Check if the sender is the owner of the contract
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Validate the user address
    let user_addr = deps.api.addr_validate(&user)?;
    // Load the user's current reputation or initialize if not present
    let mut user_reputation =
        REPUTATIONS.may_load(deps.storage, &user_addr)?.unwrap_or(UserReputation { reputation: 0 });

    // Update the user's reputation
    user_reputation.reputation = reputation;
    // Save the updated reputation in the storage
    REPUTATIONS.save(deps.storage, &user_addr, &user_reputation)?;

    // Return a response with the method and user attributes
    Ok(Response::new()
        .add_attribute("method", "update_reputation")
        .add_attribute("user", user))
}

/// The reset_reputation function allows the contract owner to reset a user's reputation.
fn reset_reputation(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    user: String,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Load the current state from the storage
    let state = STATE.load(deps.storage)?;
    // Check if the sender is the owner of the contract
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Validate the user address
    let user_addr = deps.api.addr_validate(&user)?;
    // Remove the user's reputation from the storage
    REPUTATIONS.remove(deps.storage, &user_addr);

    // Return a response with the method and user attributes
    Ok(Response::new()
        .add_attribute("method", "reset_reputation")
        .add_attribute("user", user))
}

/// The transfer function allows a user to transfer a specified amount of tokens to another user.
pub fn transfer(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
    ) -> Result<Response<CoreumMsg>, ContractError> {
    // Validate the recipient address
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    let sender_addr = info.sender.clone();

     // Load the sender's balance or initialize if not present
    let mut sender_balance =
        BALANCES.may_load(deps.storage, &sender_addr)?.unwrap_or(Uint128::zero());
    if sender_balance < amount {
        return Err(ContractError::InsufficientBalance {});
    }
    
    sender_balance = sender_balance.checked_sub(amount).map_err(|_| ContractError::Overflow {})?;
    BALANCES.save(deps.storage, &sender_addr, &sender_balance)?;
    
    let mut recipient_balance =  BALANCES.may_load(deps.storage, &recipient_addr)?.unwrap_or(Uint128::zero());
    recipient_balance = recipient_balance.checked_add(amount).map_err(|_| ContractError::Overflow {})?;
    BALANCES.save(deps.storage, &recipient_addr, &recipient_balance)?;
     // Return a response with the method, from, to, and amount attributes

    Ok(Response::new()
        .add_attribute("method", "transfer")
        .add_attribute("from", sender_addr.to_string())
        .add_attribute("to", recipient)
        .add_attribute("amount", amount.to_string()))
    }

/// The query function handles different query messages and returns the corresponding data.
#[entry_point]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Token {} => token(deps),
        QueryMsg::GetReputation { user } => query_reputation(deps, user),
        QueryMsg::GetBalance { user } => query_balance(deps, user),
    }
}

/// The token function queries and returns the details of the token issued by the contract.
fn token(deps: Deps<CoreumQueries>) -> StdResult<Binary> {
    // Load the current state from the storage
    let state = STATE.load(deps.storage)?;
    // Prepare a query request to get the token details using Coreum SDK
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetFT(assetft::Query::Token { denom: state.denom }).into();
    // Query the token details
    let res: assetft::TokenResponse = deps.querier.query(&request)?;
    // Return the token details as binary
    to_binary(&res)
}

/// The query_reputation function queries and returns the reputation of a specified user.
fn query_reputation(deps: Deps<CoreumQueries>, user: String) -> StdResult<Binary> {
    // Validate the user address
    let user_addr = deps.api.addr_validate(&user)?;
    // Load the user's reputation or initialize if not present
    let reputation =
        REPUTATIONS.may_load(deps.storage, &user_addr)?.unwrap_or(UserReputation { reputation: 0 });
    // Return the user's reputation as binary
    to_binary(&reputation)
}

/// The query_balance function queries and returns the token balance of a specified user.
fn query_balance(deps: Deps<CoreumQueries>, user: String) -> StdResult<Binary> {
    // Validate the user address
    let user_addr = deps.api.addr_validate(&user)?;
    // Load the user's balance or initialize if not present
    let balance = BALANCES.may_load(deps.storage, &user_addr)?.unwrap_or(Uint128::zero());
    // Return the user's balance as binary
    to_binary(&balance)
}