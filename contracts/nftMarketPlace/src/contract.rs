use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{SaleInfo, State, EDITIONS, NFT, NFTS, RENTALS, SALES, STATE};
use coreum_wasm_sdk::{assetft, core::{CoreumMsg, CoreumQueries}};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, CosmosMsg, BankMsg, Coin, StdError,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "nft-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the contract with owner and marketplace address
#[entry_point]
pub fn instantiate(
    deps: DepsMut<CoreumQueries>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Save the contract state
    let state = State {
        owner: deps.api.addr_validate(&msg.owner)?,
        marketplace: deps.api.addr_validate(&msg.marketplace)?,
    };
    STATE.save(deps.storage, &state)?;

    // Set the contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string()))
}

/// Execute contract functions based on the message type
#[entry_point]
pub fn execute(
    deps: DepsMut<CoreumQueries>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<CoreumMsg>, ContractError> {
    match msg {
        ExecuteMsg::CreateNFT { id, metadata, royalties } => create_nft(deps, info, id, metadata, royalties),
        ExecuteMsg::ListForSale { id, price } => list_for_sale(deps, info, id, price),
        ExecuteMsg::BuyNFT { id } => buy_nft(deps, info, id),
        ExecuteMsg::RentNFT { id, duration } => rent_nft(deps, info, id, duration),
        ExecuteMsg::ReturnNFT { id } => return_nft(deps, info, id),
        ExecuteMsg::MintEdition { id, edition } => mint_edition(deps, info, id, edition),
        ExecuteMsg::UpdateNFT { id, new_metadata } => update_nft(deps, info, id, new_metadata),
        ExecuteMsg::WithdrawFunds {} => withdraw_funds(deps, info),
    }
}

/// Create a new NFT with specified metadata and optional royalties
fn create_nft(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
    metadata: String,
    royalties: Option<u64>,
) -> Result<Response<CoreumMsg>, ContractError> {
    let nft = NFT {
        id: id.clone(),
        owner: info.sender.clone(),
        metadata,
        royalties,
    };
    NFTS.save(deps.storage, id.clone(), &nft)?;
    Ok(Response::new()
        .add_attribute("method", "create_nft")
        .add_attribute("nft_id", id))
}

/// List an NFT for sale with a specified price
fn list_for_sale(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
    price: Uint128,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Load the NFT from storage
    let nft = NFTS.load(deps.storage, id.clone())?;
    
    // Ensure the sender is the owner of the NFT
    if nft.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Save the sale information
    let sale_info = SaleInfo {
        price,
        royalty: nft.royalties,
    };
    SALES.save(deps.storage, id.clone(), &sale_info)?;

    Ok(Response::new()
        .add_attribute("method", "list_for_sale")
        .add_attribute("nft_id", id)
        .add_attribute("price", price.to_string()))
}

/// Buy an NFT that is listed for sale
fn buy_nft(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
) -> Result<Response<CoreumMsg>, ContractError> {
    // Load the sale information from storage
    let sale_info = SALES.load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidNFT {})?;
    
    // Load the NFT from storage
    let mut nft = NFTS.load(deps.storage, id.clone())?;

    // Ensure the buyer has sent enough funds
    let sent_funds = info.funds.iter().find(|c| c.denom == "uscrt").map(|c| c.amount).unwrap_or(Uint128::zero());
    if sent_funds < sale_info.price {
        return Err(ContractError::InsufficientBalance {});
    }

    // Handle the royalty payment if applicable
    let mut messages: Vec<CosmosMsg<CoreumMsg>> = vec![];
    let royalty_amount = if let Some(royalty) = sale_info.royalty {
        let royalty_amount = sale_info.price.multiply_ratio(royalty, 100u128);
        let royalty_msg = BankMsg::Send {
            to_address: nft.owner.clone().into(),
            amount: vec![Coin {
                denom: "uscrt".to_string(),
                amount: royalty_amount,
            }],
        };
        messages.push(CosmosMsg::Bank(royalty_msg));
        royalty_amount
    } else {
        Uint128::zero()
    };

    // Transfer the remaining amount to the seller
    let seller_payment = sale_info.price.checked_sub(royalty_amount)
        .map_err(|_| ContractError::Overflow {})?;
    let seller_msg = BankMsg::Send {
        to_address: nft.owner.clone().into(),
        amount: vec![Coin {
            denom: "uscrt".to_string(),
            amount: seller_payment,
        }],
    };
    messages.push(CosmosMsg::Bank(seller_msg));

    // Update the NFT owner
    nft.owner = info.sender.clone();
    NFTS.save(deps.storage, id.clone(), &nft)?;

    // Remove the sale information
    SALES.remove(deps.storage, id.clone());

    Ok(Response::new()
        .add_attribute("method", "buy_nft")
        .add_attribute("nft_id", id)
        .add_attribute("buyer", info.sender.to_string())
        .add_messages(messages))
}


/// Rent an NFT for a specified duration
fn rent_nft(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
    duration: u64,
) -> Result<Response<CoreumMsg>, ContractError> {
    let nft = NFTS.load(deps.storage, id.clone())?;
    if nft.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    RENTALS.save(deps.storage, id.clone(), &(info.sender.clone(), duration))?;
    Ok(Response::new()
        .add_attribute("method", "rent_nft")
        .add_attribute("nft_id", id)
        .add_attribute("renter", info.sender.to_string())
        .add_attribute("duration", duration.to_string()))
}

/// Return a rented NFT
fn return_nft(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
) -> Result<Response<CoreumMsg>, ContractError> {
    let rental_info = RENTALS.load(deps.storage, id.clone())?;
    if rental_info.0 != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    RENTALS.remove(deps.storage, id.clone());
    Ok(Response::new()
        .add_attribute("method", "return_nft")
        .add_attribute("nft_id", id))
}

/// Mint a limited edition of an existing NFT
fn mint_edition(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
    edition: u32,
) -> Result<Response<CoreumMsg>, ContractError> {
    let nft = NFTS.load(deps.storage, id.clone())?;
    if nft.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    EDITIONS.save(deps.storage, id.clone(), &edition)?;
    Ok(Response::new()
        .add_attribute("method", "mint_edition")
        .add_attribute("nft_id", id)
        .add_attribute("edition", edition.to_string()))
}

/// Update the metadata of an existing NFT
fn update_nft(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
    id: String,
    new_metadata: String,
) -> Result<Response<CoreumMsg>, ContractError> {
    let mut nft = NFTS.load(deps.storage, id.clone())?;
    if nft.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    nft.metadata = new_metadata;
    NFTS.save(deps.storage, id.clone(), &nft)?;
    Ok(Response::new()
        .add_attribute("method", "update_nft")
        .add_attribute("nft_id", id))
}
/// Withdraw accumulated funds from the contract
fn withdraw_funds(
    deps: DepsMut<CoreumQueries>,
    info: MessageInfo,
) -> Result<Response<CoreumMsg>, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Query the contract's balance
    let balance = deps.querier.query_balance(&state.owner, "uscrt")?;
    let withdraw_msg = BankMsg::Send {
        to_address: state.owner.into(),
        amount: vec![balance],
    };

    Ok(Response::new()
        .add_attribute("method", "withdraw_funds")
        .add_message(CosmosMsg::Bank(withdraw_msg)))
}

/// Query contract data based on the query message type
#[entry_point]
pub fn query(deps: Deps<CoreumQueries>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetNFT { id } => to_binary(&query_nft(deps, id)?),
        QueryMsg::GetNFTPrice { id } => to_binary(&query_nft_price(deps, id)?),
        QueryMsg::GetRentalInfo { id } => to_binary(&query_rental_info(deps, id)?),
    }
}

/// Query information about a specific NFT
fn query_nft(deps: Deps<CoreumQueries>, id: String) -> StdResult<NFT> {
    let nft = NFTS.load(deps.storage, id)?;
    Ok(nft)
}

/// Query the price of a specific NFT
fn query_nft_price(deps: Deps<CoreumQueries>, id: String) -> StdResult<Uint128> {
    // Placeholder implementation for querying NFT price
    Ok(Uint128::zero())
}

/// Query rental information for a specific NFT
fn query_rental_info(deps: Deps<CoreumQueries>, id: String) -> StdResult<(Addr, u64)> {
    let rental_info = RENTALS.load(deps.storage, id)?;
    Ok(rental_info)
}

/// Custom contract error types
#[derive(Debug, PartialEq)]
pub enum ContractError {
    Unauthorized {},
    Std(StdError),
    InsufficientBalance {},
    Overflow {},
    InvalidNFT {},
}

impl From<StdError> for ContractError {
    fn from(err: StdError) -> ContractError {
        ContractError::Std(err)
    }
}