use anchor_lang::prelude::*;

use crate::{
    constants::MAX_PERFORMANCE_PACKAGES,
    error::ErrorCode,
    state::{Kickstarter, KickstarterState},
};

#[derive(Accounts)]
pub struct ConfigurePerformancePackage<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin,
        constraint = kickstarter.state == KickstarterState::Initialized @ ErrorCode::InvalidKickstarterState,
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(
    ctx: Context<ConfigurePerformancePackage>,
    index: u8,
    multiplier: u8,
    allocation: u64,
) -> Result<()> {
    require!(allocation > 0, ErrorCode::InvalidPerformancePackageAllocation);
    require!(multiplier > 0, ErrorCode::InvalidPerformancePackageAllocation);

    let index_usize = index as usize;
    require!(
        index_usize < MAX_PERFORMANCE_PACKAGES,
        ErrorCode::InvalidPerformancePackageIndex
    );

    let kickstarter = &mut ctx.accounts.kickstarter;
    let new_total = kickstarter
        .configured_performance_tokens
        .checked_add(allocation)
        .ok_or(ErrorCode::PerformancePoolExceeded)?;
    require!(
        new_total <= kickstarter.performance_pool_base_tokens,
        ErrorCode::PerformancePoolExceeded
    );

    let package = &mut kickstarter.performance_packages[index_usize];
    require!(
        !package.is_configured,
        ErrorCode::PerformancePackageAlreadyConfigured
    );

    package.multiplier = multiplier;
    package.allocation = allocation;
    package.is_configured = true;
    package.is_unlocked = false;
    package.is_claimed = false;

    kickstarter.configured_performance_tokens = new_total;

    Ok(())
}

