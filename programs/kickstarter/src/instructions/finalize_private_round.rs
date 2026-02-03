use anchor_lang::prelude::*;

use crate::state::{Kickstarter, KickstarterState, PrivateFundState};
use crate::error::ErrorCode;
use crate::constants::SEED_PRIVATE_STATE;

#[derive(Accounts)]
pub struct FinalizePrivateRound<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::Unauthorized
    )]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        mut,
        seeds = [SEED_PRIVATE_STATE.as_bytes(), kickstarter.key().as_ref()],
        bump,
        has_one = kickstarter
    )]
    pub private_state: Account<'info, PrivateFundState>,
}

pub fn handler(
    ctx: Context<FinalizePrivateRound>,
    final_commitments_root: [u8; 32],
    attested_total_amount: u64,
    attestation_signature: [u8; 64], // Simplified - in production this should be verified
) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;
    let private_state = &ctx.accounts.private_state;

    require!(kickstarter.state == KickstarterState::Live, ErrorCode::InvalidKickstarterState);

    require!(
        private_state.commitments_root == final_commitments_root,
        ErrorCode::InvalidCommitmentsRoot
    );

    require!(
        private_state.committed_amount == attested_total_amount,
        ErrorCode::InvalidAttestedAmount
    );

    kickstarter.is_private_round_active = false;

    msg!(
        "Private round finalized: root={:?}, total_amount={}, investors={}",
        final_commitments_root,
        attested_total_amount,
        private_state.investor_count
    );

    Ok(())
}