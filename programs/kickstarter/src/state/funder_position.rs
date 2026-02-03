use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct FunderPosition {
    pub kickstarter: Pubkey,
    pub user: Pubkey,
    pub committed_amount: u64,
    pub accepted_amount: u64,
    pub already_claimed_base: u64,
    pub claimed_refund: u64,
    pub bump: u8,
}




