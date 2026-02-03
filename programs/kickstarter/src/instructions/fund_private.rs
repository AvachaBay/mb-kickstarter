use anchor_lang::prelude::*;
use sha2::{Digest, Sha256};
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

use crate::state::{Kickstarter, KickstarterState, PrivateFundState};
use crate::error::ErrorCode;
use crate::constants::SEED_PRIVATE_STATE;

#[commit]
#[derive(Accounts)]
pub struct FundPrivate<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        mut,
        seeds = [SEED_PRIVATE_STATE.as_bytes(), kickstarter.key().as_ref()],
        bump,
        has_one = kickstarter
    )]
    pub private_state: Account<'info, PrivateFundState>,
}

pub fn handler(ctx: Context<FundPrivate>, amount: u64, salt: [u8; 32]) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let private_state = &mut ctx.accounts.private_state;


    require!(kickstarter.state == KickstarterState::Live, ErrorCode::InvalidKickstarterState);
    require!(kickstarter.is_private_round_active, ErrorCode::InvalidKickstarterState);

    if let Some(closed_time) = kickstarter.unix_timestamp_closed {
        if Clock::get()?.unix_timestamp < closed_time {
            return err!(ErrorCode::TooEarlyToCompleteKickstarter);
        }
    }

    if private_state.committed_amount.checked_add(amount) > Some(kickstarter.hard_cap) {
        return err!(ErrorCode::OverHardcapLimit);
    }
    private_state.committed_amount = private_state.committed_amount.checked_add(amount).unwrap();

    let mut hasher = Sha256::new();
    hasher.update(ctx.accounts.funder.key().as_ref());
    hasher.update(&amount.to_le_bytes());
    hasher.update(&salt);
    let commitment_hash: [u8; 32] = hasher.finalize().into();

    let mut new_root = [0u8; 32];
    if private_state.investor_count == 0 {
        new_root = commitment_hash;
    } else {
        let mut hasher = Sha256::new();
        hasher.update(private_state.commitments_root);
        hasher.update(commitment_hash);
        new_root = hasher.finalize().into();
    }
    private_state.commitments_root = new_root;

    private_state.investor_count = private_state.investor_count.checked_add(1).unwrap();

    msg!("Private funding: commitment added, total committed: {}", private_state.committed_amount);

    commit_and_undelegate_accounts(
        &ctx.accounts.funder,
        vec![&ctx.accounts.private_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    Ok(())
}