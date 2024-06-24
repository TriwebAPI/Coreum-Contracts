use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, CustomMsg, RequestFlashLoan, RepayFlashLoan};
use crate::state::{State, STATE};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg, BankMsg, Coin, StdError,
};
use cw2::set_contract_version;
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries};

const CONTRACT_NAME: &str = "flash-loan";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the contract with the given state and save it in storage.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Set the contract version in storage
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Create a new state object with the provided owner and lending pool addresses
    let state = State {
        owner: deps.api.addr_validate(&msg.owner)?,
        lending_pool: deps.api.addr_validate(&msg.lending_pool)?,
    };

    // Save the state in storage
    STATE.save(deps.storage, &state)?;

    // Return a response with attributes
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

/// Handle execute messages and route them to the appropriate function.
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<CustomMsg>, ContractError> {
    match msg {
        // Route RequestFlashLoan message
        ExecuteMsg::RequestFlashLoan { token, amount, collateral } => request_flash_loan(deps, info, token, amount, collateral),
        // Route ExecuteOperation message
        ExecuteMsg::ExecuteOperation { token, amount, premium } => execute_operation(deps, info, token, amount, premium),
        // Route Withdraw message
        ExecuteMsg::Withdraw { token } => withdraw(deps, info, token),
    }
}

/// Handle a request for a flash loan.
pub fn request_flash_loan(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
    amount: Uint128,
    collateral: Uint128,
) -> Result<Response<CustomMsg>, ContractError> {
    // Load the contract state
    let state = STATE.load(deps.storage)?;

    // Transfer collateral to the contract
    let collateral_transfer = BankMsg::Send {
        to_address: state.lending_pool.clone().into(),
        amount: vec![Coin { denom: token.clone(), amount: collateral }],
    };

    // Create a custom flash loan request message
    let flash_loan_request = CustomMsg::RequestFlashLoan(RequestFlashLoan {
        recipient: info.sender.to_string(),
        token: token.clone(),
        amount,
    });

    // Return a response with the transfer and custom messages
    Ok(Response::new()
        .add_attribute("method", "request_flash_loan")
        .add_message(CosmosMsg::Bank(collateral_transfer))
        .add_message(CosmosMsg::Custom(flash_loan_request)))
}

/// Execute the flash loan operation, ensuring repayment with premium.
pub fn execute_operation(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
    amount: Uint128,
    premium: Uint128,
) -> Result<Response<CustomMsg>, ContractError> {
    // Load the contract state
    let _state = STATE.load(deps.storage)?;

    // Calculate the total repayment amount
    let repay_amount = amount + premium;

    // Create a custom repay flash loan message
    let repay_msg = CustomMsg::RepayFlashLoan(RepayFlashLoan {
        sender: info.sender.to_string(),
        token: token.clone(),
        amount: repay_amount,
    });

    // Query the sender's balance to ensure sufficient funds
    let balance = deps.querier.query_balance(&info.sender, &token)?;
    if balance.amount < repay_amount {
        return Err(ContractError::Std(StdError::generic_err("Insufficient funds to repay loan with premium")));
    }

    // Return the collateral if the loan is repaid
    let return_collateral = BankMsg::Send {
        to_address: info.sender.into(),
        amount: vec![Coin { denom: token.clone(), amount: repay_amount }],
    };

    // Return a response with the repay and collateral return messages
    Ok(Response::new()
        .add_attribute("method", "execute_operation")
        .add_message(CosmosMsg::Custom(repay_msg))
        .add_message(CosmosMsg::Bank(return_collateral)))
}

/// Withdraw the specified token's balance if the sender is the contract owner.
fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
) -> Result<Response<CustomMsg>, ContractError> {
    // Load the contract state
    let state = STATE.load(deps.storage)?;

    // Ensure the sender is the contract owner
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Query the owner's balance for the specified token
    let balance = deps.querier.query_balance(&state.owner, &token)?;

    // Create a withdraw message
    let withdraw_msg = BankMsg::Send {
        to_address: state.owner.into(),
        amount: vec![balance],
    };

    // Return a response with the withdraw message
    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_message(CosmosMsg::Bank(withdraw_msg)))
}

/// Handle query messages and route them to the appropriate function.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Route LoanInfo query
        QueryMsg::LoanInfo {} => loan_info(deps),
        // Route GetBalance query
        QueryMsg::GetBalance { token } => query_balance(deps, token),
    }
}

/// Query and return the current state of the loan.
fn loan_info(deps: Deps<CoreumQueries>) -> StdResult<Binary> {
    // Load the contract state
    let state = STATE.load(deps.storage)?;

    // Return the state as binary
    to_binary(&state)
}

/// Query and return the balance of the specified token.
fn query_balance(deps: Deps<CoreumQueries>, token: String) -> StdResult<Binary> {
    // Validate the token address
    let balance = deps.querier.query_balance(&deps.api.addr_validate(&token)?, &token)?;

    // Return the balance amount as binary
    to_binary(&balance.amount)
}