use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};

use crate::{
    constants::{MAX_PERFORMANCE_PACKAGES, SEED_BASE_VAULT},
    error::ErrorCode,
    state::{Kickstarter, KickstarterState},
};

#[derive(Accounts)]
pub struct ClaimPerformancePackage<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin,
    )]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        mut,
        address = kickstarter.base_vault,
        seeds = [SEED_BASE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub base_vault: InterfaceAccount<'info, SplTokenAccount>,
    #[account(mut)]
    pub recipient_base_account: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPerformancePackage>, index: u8) -> Result<()> {
    let index_usize = index as usize;
    require!(
        index_usize < MAX_PERFORMANCE_PACKAGES,
        ErrorCode::InvalidPerformancePackageIndex
    );

    let kickstarter = &mut ctx.accounts.kickstarter;
    require!(
        kickstarter.state == KickstarterState::Complete,
        ErrorCode::InvalidKickstarterState
    );

    {
        let package = &kickstarter.performance_packages[index_usize];
        require!(
            package.is_configured,
            ErrorCode::PerformancePackageNotConfigured
        );
        require!(package.is_unlocked, ErrorCode::PerformancePackageLocked);
        require!(
            !package.is_claimed,
            ErrorCode::PerformancePackageAlreadyClaimed
        );
    }

    let amount = kickstarter.performance_packages[index_usize].allocation;
    let admin_key = kickstarter.kickstarter_authority;
    let base_mint_key = kickstarter.base_mint;
    let bump = kickstarter.pda_bump;
    let bump_bytes = [bump];
    let seeds: [&[u8]; 4] = [b"kickstarter", admin_key.as_ref(), base_mint_key.as_ref(), &bump_bytes];
    let signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.base_vault.to_account_info(),
            to: ctx.accounts
                .recipient_base_account
                .to_account_info(),
            authority: kickstarter.to_account_info(),
        },
        signer,
    );
    token::transfer(cpi_ctx, amount)?;

    let package = &mut kickstarter.performance_packages[index_usize];
    package.is_claimed = true;

    Ok(())
}

