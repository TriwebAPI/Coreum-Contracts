use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, HumanAddr, ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw0::{Duration, Expiration};

pub const KEY_INVESTMENT: &[u8] = b"invest";
pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";

pub const PREFIX_CLAIMS: &[u8] = b"claim";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Claim {
    pub amount: Uint128,
    pub released: Expiration,
}

/// claims are the claims to money being unbonded, index by claimer address
pub fn claims<S: Storage>(storage: &mut S) -> Bucket<S, Vec<Claim>> {
    bucket(PREFIX_CLAIMS, storage)
}

pub fn claims_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Vec<Claim>> {
    bucket_read(PREFIX_CLAIMS, storage)
}

/// Investment info is fixed at initialization, and is used to control the function of the contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InvestmentInfo {
    /// owner created the contract and takes a cut
    pub owner: CanonicalAddr,
    /// this is the denomination we can stake (and only one we accept for payments)
    pub bond_denom: String,
    /// This is the unbonding period of the native staking module
    /// We need this to only allow claims to be redeemed after the money has arrived
    pub unbonding_period: Duration,
    /// this is how much the owner takes as a cut when someone unbonds
    pub exit_tax: Decimal,
    /// All tokens are bonded to this validator
    /// FIXME: humanize/canonicalize address doesn't work for validator addrresses
    pub validator: HumanAddr,
    /// This is the minimum amount we will pull out to reinvest, as well as a minumum
    /// that can be unbonded (to avoid needless staking tx)
    pub min_withdrawal: Uint128,
}

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Supply {
    /// issued is how many derivative tokens this contract has issued
    pub issued: Uint128,
    /// bonded is how many native tokens exist bonded to the validator
    pub bonded: Uint128,
    /// claims is how many tokens need to be reserved paying back those who unbonded
    pub claims: Uint128,
}

pub fn invest_info<S: Storage>(storage: &mut S) -> Singleton<S, InvestmentInfo> {
    singleton(storage, KEY_INVESTMENT)
}

pub fn invest_info_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, InvestmentInfo> {
    singleton_read(storage, KEY_INVESTMENT)
}

pub fn total_supply<S: Storage>(storage: &mut S) -> Singleton<S, Supply> {
    singleton(storage, KEY_TOTAL_SUPPLY)
}

pub fn total_supply_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Supply> {
    singleton_read(storage, KEY_TOTAL_SUPPLY)
}