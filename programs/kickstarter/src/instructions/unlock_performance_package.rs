use anchor_lang::prelude::*;

use crate::{
    constants::MAX_PERFORMANCE_PACKAGES,
    error::ErrorCode,
    state::{Kickstarter, KickstarterState},
};

#[derive(Accounts)]
pub struct UnlockPerformancePackage<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin,
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(ctx: Context<UnlockPerformancePackage>, index: u8, current_price: u64) -> Result<()> {
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

    let complete_time = kickstarter
        .unix_timestamp_closed
        .ok_or(ErrorCode::InvalidKickstarterState)?;
    
    let current_time = Clock::get()?.unix_timestamp;
    let unlock_delay = kickstarter.package_unlock_delay_seconds;

    if index_usize == 0 {
        let earliest_unlock = complete_time
            .checked_add(unlock_delay)
            .ok_or(ErrorCode::MathOverflow)?;
        require!(
            current_time >= earliest_unlock,
            ErrorCode::TooEarlyToUnlockPackage
        );
    } else {
        let prev_package = &kickstarter.performance_packages[index_usize - 1];
        require!(
            prev_package.is_unlocked,
            ErrorCode::PreviousPackageNotUnlocked
        );
        
        let prev_unlock_time = prev_package
            .unlocked_at
            .ok_or(ErrorCode::PreviousPackageNotUnlocked)?;
        
        let earliest_unlock = prev_unlock_time
            .checked_add(unlock_delay)
            .ok_or(ErrorCode::MathOverflow)?;
        require!(
            current_time >= earliest_unlock,
            ErrorCode::TooEarlyToUnlockPackage
        );
    }

    let initial_price = kickstarter
        .initial_token_price
        .ok_or(ErrorCode::InitialTokenPriceNotSet)?;

    let package = &kickstarter.performance_packages[index_usize];
    require!(
        package.is_configured,
        ErrorCode::PerformancePackageNotConfigured
    );
    require!(
        !package.is_unlocked,
        ErrorCode::PerformancePackageAlreadyUnlocked
    );

    let target_price_u128 = (initial_price as u128)
        .checked_mul(package.multiplier as u128)
        .ok_or(ErrorCode::MathOverflow)?;
    let target_price = u64::try_from(target_price_u128).map_err(|_| ErrorCode::MathOverflow)?;

    require!(
        current_price >= target_price,
        ErrorCode::PriceTargetNotReached
    );

    let package_mut = &mut kickstarter.performance_packages[index_usize];
    package_mut.is_unlocked = true;
    package_mut.unlocked_at = Some(current_time);

    Ok(())
}
