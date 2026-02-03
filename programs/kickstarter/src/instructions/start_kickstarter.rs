use anchor_lang::prelude::*;

use crate::state::{Kickstarter, KickstarterState};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct StartKickstarter<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(ctx: Context<StartKickstarter>) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;
    
    require!(kickstarter.state == KickstarterState::Initialized, ErrorCode::InvalidKickstarterState);
    
    kickstarter.state = KickstarterState::Live;
    kickstarter.unix_timestamp_started = Some(Clock::get()?.unix_timestamp);

    Ok(())
}
