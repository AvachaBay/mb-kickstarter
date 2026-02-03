use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct PrivateFundState {
    pub kickstarter: Pubkey,
    pub commitments_root: [u8; 32],
    pub investor_count: u32,
    pub committed_amount: u64,
    pub bump: u8,
}
