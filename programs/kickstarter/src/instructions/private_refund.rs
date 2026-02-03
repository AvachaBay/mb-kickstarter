use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};
use sha2::{Digest, Sha256};

use crate::state::{Kickstarter, KickstarterState, PrivateFundState};
use crate::error::ErrorCode;
use crate::constants::{SEED_PRIVATE_STATE, SEED_QUOTE_VAULT};

#[derive(Accounts)]
pub struct PrivateRefund<'info> {
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
        address = kickstarter.quote_vault,
        seeds = [SEED_QUOTE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub quote_vault: InterfaceAccount<'info, SplTokenAccount>,
    #[account(mut)]
    pub user_quote_account: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<PrivateRefund>,
    amount: u64,
    salt: [u8; 32]
) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let private_state = &ctx.accounts.private_state;

    require!(kickstarter.state == KickstarterState::Refunding, ErrorCode::InvalidKickstarterState);

    let mut hasher = Sha256::new();
    hasher.update(ctx.accounts.user.key().as_ref());
    hasher.update(&amount.to_le_bytes());
    hasher.update(&salt);
    let commitment_hash: [u8; 32] = hasher.finalize().into();

    // Verify commitment against current root
    let mut expected_root = [0u8; 32];
    if private_state.investor_count == 1 {
        expected_root = commitment_hash;
    } else {
        let mut hasher = Sha256::new();
        hasher.update(private_state.commitments_root);
        hasher.update(commitment_hash);
        expected_root = hasher.finalize().into();
    }

    require!(
        private_state.commitments_root == expected_root,
        ErrorCode::InvalidCommitmentsRoot
    );

    if amount > 0 {
        let seeds = &[
            b"kickstarter",
            kickstarter.kickstarter_authority.as_ref(),
            kickstarter.base_mint.as_ref(),
            &[kickstarter.pda_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.quote_vault.to_account_info(),
            to: ctx.accounts.user_quote_account.to_account_info(),
            authority: kickstarter.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer
        );
        token::transfer(cpi_ctx, amount)?;

        msg!(
            "Private refund: user={}, amount={}",
            ctx.accounts.user.key(),
            amount
        );
    }

    Ok(())
}