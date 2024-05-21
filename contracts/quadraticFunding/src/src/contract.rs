use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult,
};
use crate::error::ContractError;
use crate::helper::extract_budget_coin;
use crate::matching::{calculate_clr, QuadraticFundingAlgorithm, RawGrant};
use crate::msg::{AllProposalsResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Proposal, Vote, CONFIG, PROPOSALS, PROPOSAL_SEQ, VOTES};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate(env)?;
    let budget = extract_budget_coin(info.funds.as_slice(), &msg.budget_denom)?;
    let mut create_proposal_whitelist: Option<Vec<String>> = None;
    let mut vote_proposal_whitelist: Option<Vec<String>> = None;
    if let Some(pwl) = msg.create_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in pwl {
            deps.api.addr_validate(&w)?;
            tmp_wl.push(w);
        }
        create_proposal_whitelist = Some(tmp_wl);
    }
    if let Some(vwl) = msg.vote_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in vwl {
            deps.api.addr_validate(&w)?;
            tmp_wl.push(w);
        }
        vote_proposal_whitelist = Some(tmp_wl);
    }
    let cfg = Config {
        admin: msg.admin,
        leftover_addr: msg.leftover_addr,
        create_proposal_whitelist,
        vote_proposal_whitelist,
        voting_period: msg.voting_period,
        proposal_period: msg.proposal_period,
        algorithm: msg.algorithm,
        budget,
    };
    CONFIG.save(deps.storage, &cfg)?;
    PROPOSAL_SEQ.save(deps.storage, &0)?;
    Ok(Response::default())
}
// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateProposal {
            title,
            description,
            metadata,
            fund_address,
        } => execute_create_proposal(deps, env, info, title, description, metadata, fund_address),
        ExecuteMsg::VoteProposal { proposal_id } => {
            execute_vote_proposal(deps, env, info, proposal_id)
        }
        ExecuteMsg::TriggerDistribution { .. } => execute_trigger_distribution(deps, env, info),
    }
}
pub fn execute_create_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    metadata: Option<Binary>,
    fund_address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check whitelist
    if let Some(wl) = config.create_proposal_whitelist {
        if !wl.contains(&info.sender.to_string()) {
            return Err(ContractError::Unauthorized {});
        }
    }
    // check proposal expiration
    if config.proposal_period.is_expired(&env.block) {
        return Err(ContractError::ProposalPeriodExpired {});
    }
    // validate fund address
    deps.api.addr_validate(fund_address.as_str())?;
    let id = PROPOSAL_SEQ.load(deps.storage)? + 1;
    PROPOSAL_SEQ.save(deps.storage, &id)?;
    let p = Proposal {
        id,
        title: title.clone(),
        description,
        metadata,
        fund_address,
        ..Default::default()
    };
    PROPOSALS.save(deps.storage, id, &p)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "create_proposal"),
        attr("title", title),
        attr("proposal_id", id.to_string()),
    ]))
}
pub fn execute_vote_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check whitelist
    if let Some(wl) = config.vote_proposal_whitelist {
        if !wl.contains(&info.sender.to_string()) {
            return Err(ContractError::Unauthorized {});
        }
    }
    // check voting expiration
    if config.voting_period.is_expired(&env.block) {
        return Err(ContractError::VotingPeriodExpired {});
    }
    // validate sent funds and funding denom matches
    let fund = extract_budget_coin(&info.funds, &config.budget.denom)?;
    // check existence of the proposal and collect funds in proposal
    let proposal = PROPOSALS.update(deps.storage, proposal_id, |op| match op {
        None => Err(ContractError::ProposalNotFound {}),
        Some(mut proposal) => {
            proposal.collected_funds += fund.amount;
            Ok(proposal)
        }
    })?;
    let vote = Vote {
        proposal_id,
        voter: info.sender.to_string(),
        fund,
    };
    // check sender did not voted on proposal
    let vote_key = VOTES.key((proposal_id, info.sender.as_bytes()));
    if vote_key.may_load(deps.storage)?.is_some() {
        return Err(ContractError::AddressAlreadyVotedProject {});
    }
    // save vote
    vote_key.save(deps.storage, &vote)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "vote_proposal"),
        attr("proposal_key", proposal_id.to_string()),
        attr("voter", vote.voter),
        attr("collected_fund", proposal.collected_funds),
    ]))
}
pub fn execute_trigger_distribution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // only admin can trigger distribution
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    // check voting period expiration
    if !config.voting_period.is_expired(&env.block) {
        return Err(ContractError::VotingPeriodNotExpired {});
    }
    let query_proposals: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    let proposals: Vec<Proposal> = query_proposals?.into_iter().map(|p| p.1).collect();
    let mut grants: Vec<RawGrant> = vec![];
    // collect proposals under grants
    for p in proposals {
        let vote_query: StdResult<Vec<(Vec<u8>, Vote)>> = VOTES
            .prefix(p.id)
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        let mut votes: Vec<u128> = vec![];
        for v in vote_query? {
            votes.push(v.1.fund.amount.u128());
        }
        let grant = RawGrant {
            addr: p.fund_address,
            funds: votes,
            collected_vote_funds: p.collected_funds.u128(),
        };
        grants.push(grant);
    }
    let (distr_funds, leftover) = match config.algorithm {
        QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism { .. } => {
            calculate_clr(grants, Some(config.budget.amount.u128()))?
        }
    };
    let mut msgs = vec![];
    for f in distr_funds {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: f.addr,
            amount: vec![coin(f.grant + f.collected_vote_funds, &config.budget.denom)],
        }));
    }
    let leftover_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.leftover_addr,
        amount: vec![coin(leftover, config.budget.denom)],
    });
    msgs.push(leftover_msg);
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "trigger_distribution"))
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProposalByID { id } => to_binary(&query_proposal_id(deps, id)?),
        QueryMsg::AllProposals {} => to_binary(&query_all_proposals(deps)?),
    }
}
fn query_proposal_id(deps: Deps, id: u64) -> StdResult<Proposal> {
    PROPOSALS.load(deps.storage, id)
}
fn query_all_proposals(deps: Deps) -> StdResult<AllProposalsResponse> {
    let all: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    all.map(|p| {
        let res = p.into_iter().map(|x| x.1).collect();
        AllProposalsResponse { proposals: res }
    })
}