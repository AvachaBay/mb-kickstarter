use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};
use sha2::{Digest, Sha256};

use crate::state::{Kickstarter, KickstarterState, PrivateFundState};
use crate::error::ErrorCode;
use crate::constants::{SEED_BASE_VAULT, SEED_PRIVATE_STATE};

#[derive(Accounts)]
pub struct PrivateClaim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        seeds = [SEED_PRIVATE_STATE.as_bytes(), kickstarter.key().as_ref()],
        bump,
        has_one = kickstarter
    )]
    pub private_state: Account<'info, PrivateFundState>,
    #[account(
        mut,
        address = kickstarter.base_vault,
        seeds = [SEED_BASE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub base_vault: InterfaceAccount<'info, SplTokenAccount>,
    #[account(mut)]
    pub user_base_account: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
    // pub system_program: Program<'info, System>
}

pub fn handler(
    ctx: Context<PrivateClaim>,
    amount: u64,
    salt: [u8; 32]
) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let private_state = &ctx.accounts.private_state;

    require!(kickstarter.state == KickstarterState::Complete, ErrorCode::InvalidKickstarterState);

    let total_committed_snapshot = private_state.committed_amount;
    require!(total_committed_snapshot > 0, ErrorCode::CommittedSnapshotMissing);

    // require!(kickstarter.private_investor_count > 0, ErrorCode::InvalidCommitmentsRoot);

    let base_tokens_to_user_u128 =
        (amount as u128)
        .checked_mul(kickstarter.total_base_tokens_for_investors as u128).unwrap()
        .checked_div(total_committed_snapshot as u128).unwrap();
    let base_tokens_to_user_u64 = u64::try_from(base_tokens_to_user_u128).unwrap();

    if base_tokens_to_user_u64 > 0 {
        let seeds = &[
            b"kickstarter",
            kickstarter.kickstarter_authority.as_ref(),
            kickstarter.base_mint.as_ref(),
            &[kickstarter.pda_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.base_vault.to_account_info(),
            to: ctx.accounts.user_base_account.to_account_info(),
            authority: kickstarter.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer
        );
        token::transfer(cpi_ctx, base_tokens_to_user_u64)?;

        msg!(
            "Private claim: user={}, amount={}, tokens={}",
            ctx.accounts.user.key(),
            amount,
            base_tokens_to_user_u64
        );
    }

    Ok(())
}