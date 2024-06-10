use coreum_wasm_sdk::assetft::{
    self, BalanceResponse, FrozenBalanceResponse, FrozenBalancesResponse, ParamsResponse, Query,
    TokenResponse, TokensResponse, WhitelistedBalanceResponse, WhitelistedBalancesResponse,
};
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries, CoreumResult};
use coreum_wasm_sdk::pagination::PageRequest;
use cosmwasm_std::{coin, entry_point, to_json_binary, Binary, Deps, QueryRequest, StdResult};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use cw_ownable::{assert_owner, initialize_owner};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::DENOM;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ********** Instantiate **********

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> CoreumResult<ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    initialize_owner(deps.storage, deps.api, Some(info.sender.as_ref()))?;

    let issue_msg = CoreumMsg::AssetFT(assetft::Msg::Issue {
        symbol: msg.symbol,
        subunit: msg.subunit.clone(),
        precision: msg.precision,
        initial_amount: msg.initial_amount,
        description: msg.description,
        features: msg.features,
        burn_rate: msg.burn_rate,
        send_commission_rate: msg.send_commission_rate,
        uri: msg.uri,
        uri_hash: msg.uri_hash,
    });

    let denom = format!("{}-{}", msg.subunit, env.contract.address).to_lowercase();

    DENOM.save(deps.storage, &denom)?;

    Ok(Response::new()
        .add_attribute("owner", info.sender)
        .add_attribute("denom", denom)
        .add_message(issue_msg))
}

// ********** Execute **********

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> CoreumResult<ContractError> {
    match msg {
        ExecuteMsg::Mint { amount, recipient } => mint(deps, info, amount, recipient),
        ExecuteMsg::Burn { amount } => burn(deps, info, amount),
        ExecuteMsg::Freeze { account, amount } => freeze(deps, info, account, amount),
        ExecuteMsg::Unfreeze { account, amount } => unfreeze(deps, info, account, amount),
        ExecuteMsg::SetFrozen { account, amount } => set_frozen(deps, info, account, amount),
        ExecuteMsg::GloballyFreeze {} => globally_freeze(deps, info),
        ExecuteMsg::GloballyUnfreeze {} => globally_unfreeze(deps, info),
        ExecuteMsg::SetWhitelistedLimit { account, amount } => {
            set_whitelisted_limit(deps, info, account, amount)
        }
        ExecuteMsg::UpgradeTokenV1 { ibc_enabled } => upgrade_token_v1(deps, info, ibc_enabled),
    }
}

// ********** Transactions **********

// Function to mint the token
fn mint(deps: DepsMut, info: MessageInfo, amount: u128, recipient: Option<String>) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;
    let msg = CoreumMsg::AssetFT(assetft::Msg::Mint {
        coin: coin(amount, denom.clone()),
        recipient,
    });

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

// Function to burn the token
fn burn(deps: DepsMut, info: MessageInfo, amount: u128) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::Burn {
        coin: coin(amount, denom.clone()),
    });

    Ok(Response::new()
        .add_attribute("method", "burn")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

//Function to freeze token
fn freeze(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
    amount: u128,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::Freeze {
        account,
        coin: coin(amount, denom.clone()),
    });

    Ok(Response::new()
        .add_attribute("method", "freeze")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

//Function to unfreeze token
fn unfreeze(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
    amount: u128,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::Unfreeze {
        account,
        coin: coin(amount, denom.clone()),
    });

    Ok(Response::new()
        .add_attribute("method", "unfreeze")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

fn set_frozen(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
    amount: u128,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::SetFrozen {
        account,
        coin: coin(amount, denom.clone()),
    });

    Ok(Response::new()
        .add_attribute("method", "set_frozen")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

fn globally_freeze(deps: DepsMut, info: MessageInfo) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::GloballyFreeze {
        denom: denom.clone(),
    });

    Ok(Response::new()
        .add_attribute("method", "globally_freeze")
        .add_attribute("denom", denom)
        .add_message(msg))
}

fn globally_unfreeze(deps: DepsMut, info: MessageInfo) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::GloballyUnfreeze {
        denom: denom.clone(),
    });

    Ok(Response::new()
        .add_attribute("method", "globally_unfreeze")
        .add_attribute("denom", denom)
        .add_message(msg))
}

fn set_whitelisted_limit(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
    amount: u128,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = CoreumMsg::AssetFT(assetft::Msg::SetWhitelistedLimit {
        account,
        coin: coin(amount, denom.clone()),
    });

    Ok(Response::new()
        .add_attribute("method", "set_whitelisted_limit")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg))
}

fn upgrade_token_v1(
    deps: DepsMut,
    info: MessageInfo,
    ibc_enabled: bool,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let denom = DENOM.load(deps.storage)?;

    let upgrade_msg = CoreumMsg::AssetFT(assetft::Msg::UpgradeTokenV1 {
        denom: denom.clone(),
        ibc_enabled: ibc_enabled.clone(),
    });

    Ok(Response::new()
        .add_attribute("method", "upgrade_token_v1")
        .add_attribute("denom", denom)
        .add_attribute("ibc_enabled", ibc_enabled.to_string())
        .add_message(upgrade_msg))
}

// ********** Queries **********
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Params {} => to_json_binary(&query_params(deps)?),
        QueryMsg::Token {} => to_json_binary(&query_token(deps)?),
        QueryMsg::Tokens { issuer } => to_json_binary(&query_tokens(deps, issuer)?),
        QueryMsg::FrozenBalance { account } => to_json_binary(&query_frozen_balance(deps, account)?),
        QueryMsg::WhitelistedBalance { account } => {
            to_json_binary(&query_whitelisted_balance(deps, account)?)
        }
        QueryMsg::Balance { account } => to_json_binary(&query_balance(deps, account)?),
        QueryMsg::FrozenBalances { account } => to_json_binary(&query_frozen_balances(deps, account)?),
        QueryMsg::WhitelistedBalances { account } => {
            to_json_binary(&query_whitelisted_balances(deps, account)?)
        }
    }
}

fn query_params(deps: Deps<CoreumQueries>) -> StdResult<ParamsResponse> {
    let request = CoreumQueries::AssetFT(Query::Params {}).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}

fn query_token(deps: Deps<CoreumQueries>) -> StdResult<TokenResponse> {
    let denom = DENOM.load(deps.storage)?;
    let request = CoreumQueries::AssetFT(Query::Token { denom }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}

fn query_tokens(deps: Deps<CoreumQueries>, issuer: String) -> StdResult<TokensResponse> {
    let mut pagination = None;
    let mut tokens = vec![];
    let mut res: TokensResponse;
    loop {
        let request = CoreumQueries::AssetFT(Query::Tokens {
            pagination,
            issuer: issuer.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        tokens.append(&mut res.tokens);
        if res.pagination.next_key.is_none() {
            break;
        } else {
            pagination = Some(PageRequest {
                key: res.pagination.next_key,
                offset: None,
                limit: None,
                count_total: None,
                reverse: None,
            })
        }
    }
    let res = TokensResponse {
        pagination: res.pagination,
        tokens,
    };
    Ok(res)
}

fn query_balance(deps: Deps<CoreumQueries>, account: String) -> StdResult<BalanceResponse> {
    let denom = DENOM.load(deps.storage)?;
    let request = CoreumQueries::AssetFT(Query::Balance { account, denom }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}

fn query_frozen_balance(
    deps: Deps<CoreumQueries>,
    account: String,
) -> StdResult<FrozenBalanceResponse> {
    let denom = DENOM.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetFT(Query::FrozenBalance { denom, account }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}

fn query_frozen_balances(
    deps: Deps<CoreumQueries>,
    account: String,
) -> StdResult<FrozenBalancesResponse> {
    let mut pagination = None;
    let mut balances = vec![];
    let mut res: FrozenBalancesResponse;
    loop {
        let request = CoreumQueries::AssetFT(Query::FrozenBalances {
            pagination,
            account: account.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        balances.append(&mut res.balances);
        if res.pagination.next_key.is_none() {
            break;
        } else {
            pagination = Some(PageRequest {
                key: res.pagination.next_key,
                offset: None,
                limit: None,
                count_total: None,
                reverse: None,
            })
        }
    }
    let res = FrozenBalancesResponse {
        pagination: res.pagination,
        balances,
    };
    Ok(res)
}

fn query_whitelisted_balance(
    deps: Deps<CoreumQueries>,
    account: String,
) -> StdResult<WhitelistedBalanceResponse> {
    let denom = DENOM.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetFT(Query::WhitelistedBalance { denom, account }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}

fn query_whitelisted_balances(
    deps: Deps<CoreumQueries>,
    account: String,
) -> StdResult<WhitelistedBalancesResponse> {
    let mut pagination = None;
    let mut balances = vec![];
    let mut res: WhitelistedBalancesResponse;
    loop {
        let request = CoreumQueries::AssetFT(Query::WhitelistedBalances {
            pagination,
            account: account.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        balances.append(&mut res.balances);
        if res.pagination.next_key.is_none() {
            break;
        } else {
            pagination = Some(PageRequest {
                key: res.pagination.next_key,
                offset: None,
                limit: None,
                count_total: None,
                reverse: None,
            })
        }
    }
    let res = WhitelistedBalancesResponse {
        pagination: res.pagination,
        balances,
    };
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr, Coin, Empty};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
    use coreum_wasm_sdk::assetft::{self, MsgIssue};

    fn mock_app() -> App {
        AppBuilder::new().build()
    }

    fn contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    #[test]
    fn test_instantiate() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let res: TokenResponse = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Token {})
            .unwrap();
        
        assert_eq!(res.denom, "utest-".to_string() + contract_addr.as_str());
    }

    #[test]
    fn test_mint() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let recipient = Addr::unchecked("recipient");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let mint_msg = ExecuteMsg::Mint {
            amount: 500,
            recipient: Some(recipient.to_string()),
        };

        app.execute_contract(owner.clone(), contract_addr.clone(), &mint_msg, &[])
            .unwrap();

        let balance: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::Balance {
                    account: recipient.to_string(),
                },
            )
            .unwrap();

        assert_eq!(balance.balance.amount.u128(), 500);
    }

    #[test]
    fn test_burn() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let burn_msg = ExecuteMsg::Burn { amount: 100 };

        app.execute_contract(owner.clone(), contract_addr.clone(), &burn_msg, &[])
            .unwrap();

        let balance: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::Balance {
                    account: owner.to_string(),
                },
            )
            .unwrap();

        assert_eq!(balance.balance.amount.u128(), 900);
    }

    #[test]
    fn test_freeze_and_unfreeze() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let account = Addr::unchecked("account");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let freeze_msg = ExecuteMsg::Freeze {
            account: account.to_string(),
            amount: 100,
        };

        app.execute_contract(owner.clone(), contract_addr.clone(), &freeze_msg, &[])
            .unwrap();

        let frozen_balance: FrozenBalanceResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::FrozenBalance {
                    account: account.to_string(),
                },
            )
            .unwrap();

        assert_eq!(frozen_balance.frozen_balance.amount.u128(), 100);

        let unfreeze_msg = ExecuteMsg::Unfreeze {
            account: account.to_string(),
            amount: 50,
        };

        app.execute_contract(owner.clone(), contract_addr.clone(), &unfreeze_msg, &[])
            .unwrap();

        let frozen_balance: FrozenBalanceResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::FrozenBalance {
                    account: account.to_string(),
                },
            )
            .unwrap();

        assert_eq!(frozen_balance.frozen_balance.amount.u128(), 50);
    }

    #[test]
    fn test_globally_freeze_and_unfreeze() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let globally_freeze_msg = ExecuteMsg::GloballyFreeze {};

        app.execute_contract(owner.clone(), contract_addr.clone(), &globally_freeze_msg, &[])
            .unwrap();

        let globally_unfreeze_msg = ExecuteMsg::GloballyUnfreeze {};

        app.execute_contract(owner.clone(), contract_addr.clone(), &globally_unfreeze_msg, &[])
            .unwrap();
    }

    #[test]
    fn test_set_whitelisted_limit() {
        let mut app = mock_app();
        let contract_id = app.store_code(contract());

        let msg = InstantiateMsg {
            symbol: "TEST".to_string(),
            subunit: "utest".to_string(),
            precision: 6,
            initial_amount: 1000,
            description: "Test token".to_string(),
            features: vec![],
            burn_rate: None,
            send_commission_rate: None,
            uri: None,
            uri_hash: None,
        };

        let owner = Addr::unchecked("owner");
        let account = Addr::unchecked("account");
        let contract_addr = app
            .instantiate_contract(contract_id, owner.clone(), &msg, &[], "test", None)
            .unwrap();

        let set_whitelisted_limit_msg = ExecuteMsg::SetWhitelistedLimit {
            account: account.to_string(),
            amount: 200,
        };

        app.execute_contract(owner.clone(), contract_addr.clone(), &set_whitelisted_limit_msg, &[])
            .unwrap();

        let whitelisted_balance: WhitelistedBalanceResponse = app
            .wrap()
            .query_wasm_smart(
                contract_addr.clone(),
                &QueryMsg::WhitelistedBalance {
                    account: account.to_string(),
                },
            )
            .unwrap();

        assert_eq!(whitelisted_balance.whitelisted_balance.amount.u128(), 200);
    }
}
