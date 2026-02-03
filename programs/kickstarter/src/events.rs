use anchor_lang::prelude::*;

use crate::state::KickstarterState;

#[event]
pub struct FundEvent {
    pub kickstarter: Pubkey,
    pub funder: Pubkey,
    pub amount: u64,
    pub total_committed: u64,
}

#[event]
pub struct RefundEvent {
    pub kickstarter: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub state: KickstarterState,
}

#[event]
pub struct ClaimEvent {
    pub kickstarter: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub total_claimed: u64,
}

#[event]
pub struct CompleteEvent {
    pub kickstarter: Pubkey,
    pub state: KickstarterState,
    pub final_raise_amount: Option<u64>,
    pub liquidity_amount: u64,
    pub treasury_amount: u64,
}

#[event]
pub struct MinimumRaiseUpdatedEvent {
    pub kickstarter: Pubkey,
    pub new_minimum: u64,
}

#[event]
pub struct StakeFromTreasuryEvent {
    pub kickstarter: Pubkey,
    pub admin: Pubkey,
    pub amount: u64,
    pub staking_account: Pubkey,
}

