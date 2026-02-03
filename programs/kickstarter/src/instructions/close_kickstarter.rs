use anchor_lang::prelude::*;

use crate::state::{Kickstarter, KickstarterState};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct CloseKickstarter<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        close = admin,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(ctx: Context<CloseKickstarter>) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;
    
    require!(
        kickstarter.state == KickstarterState::Initialized || 
        kickstarter.state == KickstarterState::Complete || 
        kickstarter.state == KickstarterState::Refunding ||
        kickstarter.state == KickstarterState::Closed,
        ErrorCode::InvalidKickstarterState
    );
    
    kickstarter.unix_timestamp_closed = Some(Clock::get()?.unix_timestamp);

    Ok(())
}
