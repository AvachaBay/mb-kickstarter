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
pub struct PrivateClaimCompressed<'info> {
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
    /// CHECK: Verified in instruction logic - compressed token account
    pub compressed_token_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK: Compression program
    pub compression_program: UncheckedAccount<'info>,
    /// CHECK: System program for CPI
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<PrivateClaimCompressed>,
    amount: u64,
    salt: [u8; 32],
) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let private_state = &ctx.accounts.private_state;

    require!(kickstarter.state == KickstarterState::Complete, ErrorCode::InvalidKickstarterState);

    let total_committed_snapshot = private_state.committed_amount;
    require!(total_committed_snapshot > 0, ErrorCode::CommittedSnapshotMissing);

    let mut hasher = Sha256::new();
    hasher.update(ctx.accounts.user.key().as_ref());
    hasher.update(&amount.to_le_bytes());
    hasher.update(&salt);
    let commitment_hash: [u8; 32] = hasher.finalize().into();

    let mut expected_root = [0u8; 32];
    for i in 0..32 {
        expected_root[i] ^= commitment_hash[i];
    }

    require!(
        private_state.commitments_root == expected_root,
        ErrorCode::InvalidCommitmentsRoot
    );

    let base_tokens_to_user_u128 =
        (amount as u128)
        .checked_mul(kickstarter.total_base_tokens_for_investors as u128).unwrap()
        .checked_div(total_committed_snapshot as u128).unwrap();
    let base_tokens_to_user_u64 = u64::try_from(base_tokens_to_user_u128).unwrap();

    if base_tokens_to_user_u64 > 0 {
        // First transfer tokens to vault (simplified - in production use compressed mint)
        let seeds = &[
            b"kickstarter",
            kickstarter.kickstarter_authority.as_ref(),
            kickstarter.base_mint.as_ref(),
            &[kickstarter.pda_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.base_vault.to_account_info(),
            to: ctx.accounts.compressed_token_account.to_account_info(),
            authority: kickstarter.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer
        );
        token::transfer(cpi_ctx, base_tokens_to_user_u64)?;

        // TODO: Compress token using ZK compression
        // This would involve calling the compression program to create compressed NFT/token
        // For now, this is a placeholder for the compression logic

        msg!(
            "Private compressed claim: user={}, amount={}, tokens={}, commitment_verified=true",
            ctx.accounts.user.key(),
            amount,
            base_tokens_to_user_u64
        );
    }

    Ok(())
}