[package]
name = "nft"
version = "0.1.0"
authors = ["developer.tusharagrawal@gmail.com"]
edition = "2024"
exclude = [
    "nft.wasm",
    "checksums.txt",
]
[lib]
crate-type = ["cdylib", "rlib"]
[profile.release]
opt-level = 3
debug = false
rpath = false
lto = true
debug-assertions = false
codegen-units = 1
panic = 'abort'
incremental = false
overflow-checks = true
[features]
backtraces = ["cosmwasm-std/backtraces"]
library = []
[dependencies]
cosmwasm-std = "1.5.4"
cosmwasm-storage = "1.5.2"
cw-storage-plus = "1.2.0"
cw2 = "1.1.2"
thiserror = "1.0.59"
schemars = "0.8.16"
# TODO(keyleu): Update dependency once final version of coreum-wasm-sdk crate is pushed
coreum-wasm-sdk = { git = "https://github.com/CoreumFoundation/coreum-wasm-sdk.git", branch = "keyne/upgrade-wasm-sdk" }
cosmwasm-schema = "1.5.4"
cw-ownable = "0.5.1"
Added file
contracts/cosmos/coreum/nft/src/contract.rs


Viewed


@@ -0,0 +1,849 @@
use coreum_wasm_sdk::assetnft::{
    self, BurntNFTResponse, BurntNFTsInClassResponse, ClassFrozenAccountsResponse,
    ClassFrozenResponse, ClassResponse, ClassWhitelistedAccountsResponse, ClassesResponse,
    FrozenResponse, ParamsResponse, WhitelistedAccountsForNFTResponse, WhitelistedResponse,
};
use coreum_wasm_sdk::core::{CoreumMsg, CoreumQueries, CoreumResult};
use coreum_wasm_sdk::nft;
use coreum_wasm_sdk::pagination::PageRequest;
use coreum_wasm_sdk::types::coreum::asset::nft::v1::{
    DataBytes, DataDynamic, DataDynamicIndexedItem, DataDynamicItem, DataEditor, MsgMint,
    MsgUpdateData,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryRequest,
    Response, StdResult,
};
use cw2::set_contract_version;
use cw_ownable::{assert_owner, initialize_owner};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CLASS_ID;
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
    let issue_msg = CoreumMsg::AssetNFT(assetnft::Msg::IssueClass {
        name: msg.name,
        symbol: msg.symbol.clone(),
        description: msg.description,
        uri: msg.uri,
        uri_hash: msg.uri_hash,
        data: msg.data,
        features: msg.features,
        royalty_rate: msg.royalty_rate,
    });
    let class_id = format!("{}-{}", msg.symbol, env.contract.address).to_lowercase();
    CLASS_ID.save(deps.storage, &class_id)?;
    Ok(Response::new()
        .add_attribute("owner", info.sender)
        .add_attribute("class_id", class_id)
        .add_message(issue_msg))
}
// ********** Execute **********
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> CoreumResult<ContractError> {
    match msg {
        ExecuteMsg::MintLegacy {
            id,
            uri,
            uri_hash,
            data,
            recipient,
        } => mint_legacy(deps, info, id, uri, uri_hash, data, recipient),
        ExecuteMsg::MintImmutable {
            id,
            uri,
            uri_hash,
            data,
            recipient,
        } => mint_immutable(deps, info, env, id, uri, uri_hash, data, recipient),
        ExecuteMsg::MintMutable {
            id,
            uri,
            uri_hash,
            data,
            recipient,
        } => mint_mutable(deps, info, env, id, uri, uri_hash, data, recipient),
        ExecuteMsg::Burn { id } => burn(deps, info, id),
        ExecuteMsg::Freeze { id } => freeze(deps, info, id),
        ExecuteMsg::Unfreeze { id } => unfreeze(deps, info, id),
        ExecuteMsg::AddToWhitelist { id, account } => add_to_white_list(deps, info, id, account),
        ExecuteMsg::RemoveFromWhitelist { id, account } => {
            remove_from_white_list(deps, info, id, account)
        }
        ExecuteMsg::Send { id, receiver } => send(deps, info, id, receiver),
        ExecuteMsg::ClassFreeze { account } => class_freeze(deps, info, account),
        ExecuteMsg::ClassUnfreeze { account } => class_unfreeze(deps, info, account),
        ExecuteMsg::AddToClassWhitelist { account } => add_to_class_whitelist(deps, info, account),
        ExecuteMsg::RemoveFromClassWhitelist { account } => {
            remove_from_class_whitelist(deps, info, account)
        }
        ExecuteMsg::ModifyData { id, data } => modify_data(deps, info, env, id, data),
    }
}
// ********** Transactions **********
fn mint_legacy(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
    uri: Option<String>,
    uri_hash: Option<String>,
    data: Option<Binary>,
    recipient: Option<String>,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::Mint {
        class_id: class_id.clone(),
        id: id.clone(),
        uri,
        uri_hash,
        data,
        recipient,
    });
    Ok(Response::new()
        .add_attribute("method", "mint_legacy")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn mint_immutable(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    id: String,
    uri: Option<String>,
    uri_hash: Option<String>,
    data: Option<Binary>,
    recipient: Option<String>,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let data = match data {
        Some(data) => Some(
            DataBytes {
                data: data.to_vec(),
            }
            .to_any(),
        ),
        None => None,
    };
    let mint = MsgMint {
        sender: env.contract.address.to_string(),
        class_id: class_id.clone(),
        id: id.clone(),
        uri: uri.unwrap_or_default(),
        uri_hash: uri_hash.unwrap_or_default(),
        data,
        recipient: recipient.unwrap_or_default(),
    };
    let mint_bytes = mint.to_proto_bytes();
    let msg = CosmosMsg::Stargate {
        type_url: mint.to_any().type_url,
        value: Binary::from(mint_bytes),
    };
    Ok(Response::new()
        .add_attribute("method", "mint_immutable")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn mint_mutable(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    id: String,
    uri: Option<String>,
    uri_hash: Option<String>,
    data: Option<Binary>,
    recipient: Option<String>,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let data = match data {
        Some(data) => Some(
            DataDynamic {
                items: [DataDynamicItem {
                    editors: [DataEditor::Admin as i32, DataEditor::Owner as i32].to_vec(),
                    data: data.to_vec(),
                }]
                .to_vec(),
            }
            .to_any(),
        ),
        None => None,
    };
    let mint = MsgMint {
        sender: env.contract.address.to_string(),
        class_id: class_id.clone(),
        id: id.clone(),
        uri: uri.unwrap_or_default(),
        uri_hash: uri_hash.unwrap_or_default(),
        data,
        recipient: recipient.unwrap_or_default(),
    };
    let mint_bytes = mint.to_proto_bytes();
    let msg = CosmosMsg::Stargate {
        type_url: mint.to_any().type_url,
        value: Binary::from(mint_bytes),
    };
    Ok(Response::new()
        .add_attribute("method", "mint_mutable")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn modify_data(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    id: String,
    data: Binary,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let modify_data = MsgUpdateData {
        sender: env.contract.address.to_string(),
        class_id: class_id.clone(),
        id: id.clone(),
        items: [DataDynamicIndexedItem {
            index: 0,
            data: data.to_vec(),
        }]
        .to_vec(),
    };
    let modify_data_bytes = modify_data.to_proto_bytes();
    let msg = CosmosMsg::Stargate {
        type_url: modify_data.to_any().type_url,
        value: Binary::from(modify_data_bytes),
    };
    Ok(Response::new()
        .add_attribute("method", "modify_data")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn burn(deps: DepsMut, info: MessageInfo, id: String) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::Burn {
        class_id: class_id.clone(),
        id: id.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "burn")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn freeze(deps: DepsMut, info: MessageInfo, id: String) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::Freeze {
        class_id: class_id.clone(),
        id: id.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "freeze")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn unfreeze(deps: DepsMut, info: MessageInfo, id: String) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::Unfreeze {
        class_id: class_id.clone(),
        id: id.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "unfreeze")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn add_to_white_list(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
    account: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::AddToWhitelist {
        class_id: class_id.clone(),
        id: id.clone(),
        account,
    });
    Ok(Response::new()
        .add_attribute("method", "add_to_white_list")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn remove_from_white_list(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
    account: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::RemoveFromWhitelist {
        class_id: class_id.clone(),
        id: id.clone(),
        account,
    });
    Ok(Response::new()
        .add_attribute("method", "remove_from_white_list")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn send(
    deps: DepsMut,
    info: MessageInfo,
    id: String,
    receiver: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::NFT(nft::Msg::Send {
        class_id: class_id.clone(),
        id: id.clone(),
        receiver,
    });
    Ok(Response::new()
        .add_attribute("method", "send")
        .add_attribute("class_id", class_id)
        .add_attribute("id", id)
        .add_message(msg))
}
fn class_freeze(deps: DepsMut, info: MessageInfo, account: String) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::ClassFreeze {
        class_id: class_id.clone(),
        account: account.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "class_freeze")
        .add_attribute("class_id", class_id)
        .add_attribute("account", account)
        .add_message(msg))
}
fn class_unfreeze(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::ClassUnfreeze {
        class_id: class_id.clone(),
        account: account.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "class_unfreeze")
        .add_attribute("class_id", class_id)
        .add_attribute("account", account)
        .add_message(msg))
}
fn add_to_class_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::AddToClassWhitelist {
        class_id: class_id.clone(),
        account: account.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "add_to_class_whitelist")
        .add_attribute("class_id", class_id)
        .add_attribute("account", account)
        .add_message(msg))
}
fn remove_from_class_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    account: String,
) -> CoreumResult<ContractError> {
    assert_owner(deps.storage, &info.sender)?;
    let class_id = CLASS_ID.load(deps.storage)?;
    let msg = CoreumMsg::AssetNFT(assetnft::Msg::RemoveFromClassWhitelist {
        class_id: class_id.clone(),
        account: account.clone(),
    });
    Ok(Response::new()
        .add_attribute("method", "remove_from_class_whitelist")
        .add_attribute("class_id", class_id)
        .add_attribute("account", account)
        .add_message(msg))
}
// ********** Queries **********
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Params {} => to_json_binary(&query_params(deps)?),
        QueryMsg::Class {} => to_json_binary(&query_class(deps)?),
        QueryMsg::Classes { issuer } => to_json_binary(&query_classes(deps, issuer)?),
        QueryMsg::Frozen { id } => to_json_binary(&query_frozen(deps, id)?),
        QueryMsg::Whitelisted { id, account } => {
            to_json_binary(&query_whitelisted(deps, id, account)?)
        }
        QueryMsg::WhitelistedAccountsForNft { id } => {
            to_json_binary(&query_whitelisted_accounts_for_nft(deps, id)?)
        }
        QueryMsg::Balance { owner } => to_json_binary(&query_balance(deps, owner)?),
        QueryMsg::Owner { id } => to_json_binary(&query_owner(deps, id)?),
        QueryMsg::Supply {} => to_json_binary(&query_supply(deps)?),
        QueryMsg::Nft { id } => to_json_binary(&query_nft(deps, id)?),
        QueryMsg::Nfts { owner } => to_json_binary(&query_nfts(deps, owner)?),
        QueryMsg::ClassNft {} => to_json_binary(&query_nft_class(deps)?),
        QueryMsg::ClassesNft {} => to_json_binary(&query_nft_classes(deps)?),
        QueryMsg::BurntNft { nft_id } => to_json_binary(&query_burnt_nft(deps, nft_id)?),
        QueryMsg::BurntNftsInClass {} => to_json_binary(&query_burnt_nfts_in_class(deps)?),
        QueryMsg::ClassFrozen { account } => to_json_binary(&query_class_frozen(deps, account)?),
        QueryMsg::ClassFrozenAccounts {} => to_json_binary(&query_class_frozen_accounts(deps)?),
        QueryMsg::ClassWhitelistedAccounts {} => {
            to_json_binary(&query_class_whitelisted_accounts(deps)?)
        }
    }
}
fn query_params(deps: Deps<CoreumQueries>) -> StdResult<ParamsResponse> {
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::Params {}).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_class(deps: Deps<CoreumQueries>) -> StdResult<ClassResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::Class { id: class_id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_classes(deps: Deps<CoreumQueries>, issuer: String) -> StdResult<ClassesResponse> {
    let mut pagination = None;
    let mut classes = vec![];
    let mut res: ClassesResponse;
    loop {
        let request = CoreumQueries::AssetNFT(assetnft::Query::Classes {
            pagination,
            issuer: issuer.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        classes.append(&mut res.classes);
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
    let res = ClassesResponse {
        pagination: res.pagination,
        classes,
    };
    Ok(res)
}
fn query_frozen(deps: Deps<CoreumQueries>, id: String) -> StdResult<FrozenResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::Frozen { id, class_id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_whitelisted(
    deps: Deps<CoreumQueries>,
    id: String,
    account: String,
) -> StdResult<WhitelistedResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::Whitelisted {
            id,
            class_id,
            account,
        })
        .into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_whitelisted_accounts_for_nft(
    deps: Deps<CoreumQueries>,
    id: String,
) -> StdResult<WhitelistedAccountsForNFTResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let mut pagination = None;
    let mut accounts = vec![];
    let mut res: WhitelistedAccountsForNFTResponse;
    loop {
        let request = CoreumQueries::AssetNFT(assetnft::Query::WhitelistedAccountsForNFT {
            pagination,
            id: id.clone(),
            class_id: class_id.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        accounts.append(&mut res.accounts);
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
    let res = WhitelistedAccountsForNFTResponse {
        pagination: res.pagination,
        accounts,
    };
    Ok(res)
}
fn query_burnt_nft(deps: Deps<CoreumQueries>, nft_id: String) -> StdResult<BurntNFTResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::BurntNFT { class_id, nft_id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_burnt_nfts_in_class(deps: Deps<CoreumQueries>) -> StdResult<BurntNFTsInClassResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let mut pagination = None;
    let mut nft_ids = vec![];
    let mut res: BurntNFTsInClassResponse;
    loop {
        let request = CoreumQueries::AssetNFT(assetnft::Query::BurntNFTsInClass {
            pagination,
            class_id: class_id.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        nft_ids.append(&mut res.nft_ids);
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
    let res = BurntNFTsInClassResponse {
        pagination: res.pagination,
        nft_ids,
    };
    Ok(res)
}
fn query_class_frozen(
    deps: Deps<CoreumQueries>,
    account: String,
) -> StdResult<ClassFrozenResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::AssetNFT(assetnft::Query::ClassFrozen { class_id, account }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_class_frozen_accounts(
    deps: Deps<CoreumQueries>,
) -> StdResult<ClassFrozenAccountsResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let mut pagination = None;
    let mut accounts = vec![];
    let mut res: ClassFrozenAccountsResponse;
    loop {
        let request = CoreumQueries::AssetNFT(assetnft::Query::ClassFrozenAccounts {
            pagination,
            class_id: class_id.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        accounts.append(&mut res.accounts);
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
    let res = ClassFrozenAccountsResponse {
        pagination: res.pagination,
        accounts,
    };
    Ok(res)
}
fn query_class_whitelisted_accounts(
    deps: Deps<CoreumQueries>,
) -> StdResult<ClassWhitelistedAccountsResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let mut pagination = None;
    let mut accounts = vec![];
    let mut res: ClassWhitelistedAccountsResponse;
    loop {
        let request = CoreumQueries::AssetNFT(assetnft::Query::ClassWhitelistedAccounts {
            pagination,
            class_id: class_id.clone(),
        })
        .into();
        res = deps.querier.query(&request)?;
        accounts.append(&mut res.accounts);
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
    let res = ClassWhitelistedAccountsResponse {
        pagination: res.pagination,
        accounts,
    };
    Ok(res)
}
// ********** NFT **********
fn query_balance(deps: Deps<CoreumQueries>, owner: String) -> StdResult<nft::BalanceResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::NFT(nft::Query::Balance { class_id, owner }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_owner(deps: Deps<CoreumQueries>, id: String) -> StdResult<nft::OwnerResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::NFT(nft::Query::Owner { class_id, id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_supply(deps: Deps<CoreumQueries>) -> StdResult<nft::SupplyResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::NFT(nft::Query::Supply { class_id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_nft(deps: Deps<CoreumQueries>, id: String) -> StdResult<nft::NFTResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::NFT(nft::Query::NFT { class_id, id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_nfts(deps: Deps<CoreumQueries>, owner: Option<String>) -> StdResult<nft::NFTsResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let mut pagination = None;
    let mut nfts = vec![];
    let mut res: nft::NFTsResponse;
    if owner.is_none() {
        loop {
            let request = CoreumQueries::NFT(nft::Query::NFTs {
                class_id: Some(class_id.clone()),
                owner: None,
                pagination,
            })
            .into();
            res = deps.querier.query(&request)?;
            nfts.append(&mut res.nfts);
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
        let res = nft::NFTsResponse {
            nfts,
            pagination: res.pagination,
        };
        Ok(res)
    } else {
        loop {
            let request = CoreumQueries::NFT(nft::Query::NFTs {
                class_id: None,
                owner: Some(owner.clone().unwrap()),
                pagination,
            })
            .into();
            res = deps.querier.query(&request)?;
            nfts.append(&mut res.nfts);
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
        let res = nft::NFTsResponse {
            nfts,
            pagination: res.pagination,
        };
        Ok(res)
    }
}
fn query_nft_class(deps: Deps<CoreumQueries>) -> StdResult<nft::ClassResponse> {
    let class_id = CLASS_ID.load(deps.storage)?;
    let request: QueryRequest<CoreumQueries> =
        CoreumQueries::NFT(nft::Query::Class { class_id }).into();
    let res = deps.querier.query(&request)?;
    Ok(res)
}
fn query_nft_classes(deps: Deps<CoreumQueries>) -> StdResult<nft::ClassesResponse> {
    let mut pagination = None;
    let mut classes = vec![];
    let mut res: nft::ClassesResponse;
    loop {
        let request = CoreumQueries::NFT(nft::Query::Classes { pagination }).into();
        res = deps.querier.query(&request)?;
        classes.append(&mut res.classes);
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
    let res = nft::ClassesResponse {
        classes,
        pagination: res.pagination,
    };
    Ok(res)
}