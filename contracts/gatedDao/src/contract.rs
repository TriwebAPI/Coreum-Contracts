use cosmwasm_std::{
    entry_point, BankMsg, SubMsg, Coin, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, StdError
};
use cosmwasm_std::to_binary;
use cw2::set_contract_version;
use cosmwasm_std::{ Addr};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Proposal, Member, PROPOSAL_COUNT, PROPOSALS, MEMBERS};

const CONTRACT_NAME: &str = "workshop-dao";
const CONTRACT_VERSION: &str = "0.1.0";

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Invalid input")]
    InvalidInput(String),
    #[error("Already Executed")]
    AlreadyExecuted {},
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize proposal count with 0
    PROPOSAL_COUNT.save(deps.storage, &0u64)?;



    for member in msg.members {
        MEMBERS.save(deps.storage, member.address.as_str(), &Member {
            address: member.address.clone(),
            weight: Uint128::from(member.weight),
        })?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Propose { title, description, recipient, amount } => execute_propose(deps, env, info, title, description, recipient,amount),
        ExecuteMsg::Vote { proposal_id, approve } => execute_vote(deps, info, proposal_id, approve),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, proposal_id),  // Add env here
    }
}

fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    recipient: Option<Addr>,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let sender_addr = info.sender.as_str();
    let member_opt = MEMBERS.load(deps.storage, sender_addr);

    // This automatically returns Unauthorized if the sender is not found in MEMBERS
    if member_opt.is_err() {
        return Err(ContractError::Unauthorized {});
    }

    // Get the current proposal count and increment it for a new unique ID
    let mut proposal_count = PROPOSAL_COUNT.load(deps.storage).unwrap_or_default();
    proposal_count += 1;

    // Save the updated count back to storage
    PROPOSAL_COUNT.save(deps.storage, &proposal_count)?;



    let voting_period = 604800; // 7 days in seconds
    let proposal = Proposal {
        id: 0, // This should be a unique ID, possibly increment based on the last ID
        title,
        description,
        votes_for: Uint128::zero(),
        votes_against: Uint128::zero(),
        executed: false,
        amount: amount.unwrap_or_else(Uint128::zero),
        recipient: recipient.unwrap_or(info.sender),
        voting_end: env.block.time.seconds() + voting_period,
    };

    PROPOSALS.save(deps.storage, &proposal.id.to_string(), &proposal)?;

    Ok(Response::default().add_attribute("action", "propose"))
}

fn execute_vote(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u64,
    approve: bool,
) -> Result<Response, ContractError> {
    let sender_addr = info.sender.as_str();
    let member_opt = MEMBERS.load(deps.storage, sender_addr); 

    if member_opt.is_err() {
        return Err(ContractError::Unauthorized {});
    }
    // We are trying to extract the value if it's Some or None

    let member = member_opt.unwrap();

    let mut proposal = PROPOSALS.load(deps.storage, &proposal_id.to_string())?;

    if approve {
        proposal.votes_for += member.weight;
    } else {
        proposal.votes_against += member.weight;
    }

    PROPOSALS.save(deps.storage, &proposal_id.to_string(), &proposal)?;

    Ok(Response::default())
}

fn execute_execute(
    deps: DepsMut,
    _env: Env,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let mut proposal = PROPOSALS.load(deps.storage, &proposal_id.to_string())?;

    


    if proposal.votes_for > proposal.votes_against {
        let recipient = &proposal.recipient;
        let amount = &proposal.amount;

        let current_balance = Uint128::zero();

        if current_balance < *amount {
            return Err(ContractError::Std(StdError::generic_err("Insufficient funds")));
        }

        let transfer = BankMsg::Send {
            to_address: recipient.clone().into(),
            amount: vec![Coin {
                denom: "udevcore".to_string(),
                amount: amount.clone(),
            }],
        };

        proposal.executed = true;
        PROPOSALS.save(deps.storage, &proposal_id.to_string(), &proposal)?;

        let cosmos_msg = cosmwasm_std::CosmosMsg::Bank(transfer);

        return Ok(Response::new()
            .add_message(cosmos_msg) // TODO: use coreum message instead?
            .add_attribute("method", "execute_execute")
            .add_attribute("recipient", recipient.to_string())
            .add_attribute("amount", amount.to_string()));
    }

    Ok(Response::default())
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetProposal { proposal_id } => query_proposal(deps, proposal_id),
        QueryMsg::ListProposals {} => query_all_proposals(deps),
        QueryMsg::GetMember { address } => query_member(deps, address),
        QueryMsg::ListMembers {} => query_all_members(deps),
    }
}

fn query_proposal(deps: Deps, proposal_id: u64) -> StdResult<Binary> {
    let proposal = PROPOSALS.load(deps.storage, &proposal_id.to_string())
        .map_err(|_| StdError::not_found("Proposal"))?;
    to_binary(&proposal)
}

fn query_all_proposals(deps: Deps) -> StdResult<Binary> {
    let proposals = PROPOSALS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (_key, proposal) = item?;
            Ok(proposal)
        })
        .collect::<StdResult<Vec<Proposal>>>()?;
    to_binary(&proposals)
}

fn query_member(deps: Deps, address: Addr) -> StdResult<Binary> {
    let member = MEMBERS.load(deps.storage, address.as_str())
        .map_err(|_| StdError::not_found("Member"))?;
    to_binary(&member)
}

fn query_all_members(deps: Deps) -> StdResult<Binary> {
    let members = MEMBERS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (_key, member) = item?;
            Ok(member)
        })
        .collect::<StdResult<Vec<Member>>>()?;
    to_binary(&members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Uint128};
    use crate::state::Member;


    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();
        
        let members = vec![
            Member {
                address: Addr::unchecked("addr1"),
                weight: Uint128::from(10_u128),
            },
            Member {
                address: Addr::unchecked("addr2"),
                weight: Uint128::from(20_u128),

            },
        ];

        let msg = InstantiateMsg {members};
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proposal_creation() {
        let mut deps = mock_dependencies();

        let members = vec![
            Member {
                address: Addr::unchecked("addr1"),
                weight: Uint128::from(10_u128),
            },
        ];

        let msg = InstantiateMsg { members };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Propose
        let info = mock_info("addr1", &[]);
        let msg = ExecuteMsg::Propose {
            title: "Test Proposal".to_string(),
            description: "Description for test".to_string(),
            amount: Some(Uint128::from(100_u128)),
            recipient: Some(Addr::unchecked("recipient_address")),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn vote_for_proposal() {
        let mut deps = mock_dependencies();

        let members = vec![
            Member {
                address: Addr::unchecked("addr1"),
                weight: Uint128::from(10_u128),
            },
        ];
        let msg = InstantiateMsg { members };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Propose
        let info = mock_info("addr1", &[]);
        let proposal_msg = ExecuteMsg::Propose {
            title: "Some Title".to_string(),
            description: "Some Description".to_string(),
            amount: Some(Uint128::from(100_u128)),
            recipient: Some(Addr::unchecked("recipient_address")),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), proposal_msg).unwrap();

        let vote_msg = ExecuteMsg::Vote {
            proposal_id: 0,
            approve: true,
        };

        let res = execute(deps.as_mut(), mock_env(), info, vote_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }


    #[test]
    fn execute_proposal() {
        let mut deps = mock_dependencies();

        let members = vec![
            Member {
                address: Addr::unchecked("addr1"),
                weight: Uint128::from(10_u128),
            },
        ];
        let msg = InstantiateMsg { members };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("addr1", &[]);
        let proposal_msg = ExecuteMsg::Propose {
            title: "Another Title".to_string(),
            description: "Another Description".to_string(),
            amount: Some(Uint128::from(100_u128)),
            recipient: Some(Addr::unchecked("recipient_address")),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), proposal_msg).unwrap();

        let vote_msg = ExecuteMsg::Vote {
            proposal_id: 0,
            approve: true,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), vote_msg).unwrap();

        let exec_msg = ExecuteMsg::Execute { proposal_id: 0 };
        let res = execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();
        assert_eq!(1, res.messages.len());
    }
    
}