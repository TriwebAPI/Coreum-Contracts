use crate::msg::AmountResponse;
use coreum_wasm_sdk::assetft;
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries};
use cosmwasm_std::{entry_point, to_binary, Binary, Deps, QueryRequest, StdResult};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use cw2::set_contract_version;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};
use thiserror::Error;
// version info for migration info
const CONTRACT_NAME: &str = "creates.io:ft";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub symbol: String,
    pub subunit: String,
    pub precision: u32,
    pub initial_amount: Uint128,
    pub airdrop_amount: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: String,
    pub denom: String,
    pub airdrop_amount: Uint128,
    pub minted_for_airdrop: Uint128,
}
pub const STATE: Item<State> = Item::new("state");
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Invalid input")]
    InvalidInput(String),
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    MintForAirdrop { amount: u128 },
    ReceiveAirdrop {},
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    match msg {
        ExecuteMsg::MintForAirdrop { amount } => mint_for_airdrop(deps, info, amount),
        ExecuteMsg::ReceiveAirdrop {} => receive_airdrop(deps, info),
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Token {},
    MintedForAirdrop {},
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Token {} => token(deps),
        QueryMsg::MintedForAirdrop {} => minted_for_airdrop(deps),
    }
}
// ********** Instantiate **********
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
    let denom = format!("{}-{}", msg.subunit, env.contract.address).to_lowercase();
    let state = State {
        owner: info.sender.into(),
        denom,
        minted_for_airdrop: msg.initial_amount,
        airdrop_amount: msg.airdrop_amount,
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("owner", state.owner)
        .add_attribute("denom", state.denom)
        .add_message(issue_msg))
}
// ********** Transactions **********
fn mint_for_airdrop(
    deps: DepsMut,
    info: MessageInfo,
    amount: u128,
) -> Result<Response<CoreumMsg>, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }
    let msg = CoreumMsg::AssetFT(assetft::Msg::Mint {
        coin: Coin::new(amount, state.denom.clone()),
    });
    state.minted_for_airdrop = state.minted_for_airdrop.add(Uint128::new(amount));
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "mint_for_airdrop")
        .add_attribute("denom", state.denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}
fn receive_airdrop(deps: DepsMut, info: MessageInfo) -> Result<Response<CoreumMsg>, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if state.minted_for_airdrop < state.airdrop_amount {
        return Err(ContractError::CustomError {
            val: "not enough minted".into(),
        });
    }
    let send_msg = cosmwasm_std::BankMsg::Send {
        to_address: info.sender.into(),
        amount: vec![Coin {
            amount: state.airdrop_amount,
            denom: state.denom.clone(),
        }],
    };
    state.minted_for_airdrop = state.minted_for_airdrop.sub(state.airdrop_amount);
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "receive_airdrop")
        .add_attribute("denom", state.denom)
        .add_attribute("amount", state.airdrop_amount.to_string())
        .add_message(send_msg))
}
// ********** Queries **********
fn token(deps: Deps<CoreumQueries>) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetFT(assetft::Query::Token { denom: state.denom }).into();
    let res: assetft::TokenResponse = deps.querier.query(&request)?;
    to_binary(&res)
}
fn minted_for_airdrop(deps: Deps<CoreumQueries>) -> StdResult<Binary> {
    let state = STATE.load(deps.storage)?;
    let res = AmountResponse {
        amount: state.minted_for_airdrop,
    };
    to_binary(&res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes, vec![
            attr("owner", "creator"),
            attr("denom", "test-0x0000000000000000000000000000000000000000")
        ]);
    }

    #[test]
    fn mint_for_airdrop_authorized() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let mint_msg = ExecuteMsg::MintForAirdrop { amount: 500 };
        let res = execute(deps.as_mut(), mock_env(), info, mint_msg).unwrap();

        assert_eq!(res.attributes, vec![
            attr("method", "mint_for_airdrop"),
            attr("denom", "test-0x0000000000000000000000000000000000000000"),
            attr("amount", "500")
        ]);

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(state.minted_for_airdrop, Uint128::new(1500));
    }

    #[test]
    fn mint_for_airdrop_unauthorized() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let unauthorized_info = mock_info("not_creator", &[]);
        let mint_msg = ExecuteMsg::MintForAirdrop { amount: 500 };
        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, mint_msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn receive_airdrop() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let mint_msg = ExecuteMsg::MintForAirdrop { amount: 500 };
        execute(deps.as_mut(), mock_env(), info.clone(), mint_msg).unwrap();

        let receive_msg = ExecuteMsg::ReceiveAirdrop {};
        let res = execute(deps.as_mut(), mock_env(), mock_info("recipient", &[]), receive_msg).unwrap();

        assert_eq!(res.attributes, vec![
            attr("method", "receive_airdrop"),
            attr("denom", "test-0x0000000000000000000000000000000000000000"),
            attr("amount", "100")
        ]);

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(state.minted_for_airdrop, Uint128::new(1400));
    }

    #[test]
    fn query_token() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let query_msg = QueryMsg::Token {};
        let bin = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let token_response: assetft::TokenResponse = from_binary(&bin).unwrap();

        assert_eq!(token_response.denom, "test-0x0000000000000000000000000000000000000000");
    }

    #[test]
    fn query_minted_for_airdrop() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "test".to_string(),
            precision: 6,
            initial_amount: Uint128::new(1000),
            airdrop_amount: Uint128::new(100),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let query_msg = QueryMsg::MintedForAirdrop {};
        let bin = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let amount_response: AmountResponse = from_binary(&bin).unwrap();

        assert_eq!(amount_response.amount, Uint128::new(1000));
    }
}
