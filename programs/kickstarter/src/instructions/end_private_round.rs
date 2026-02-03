use anchor_lang::prelude::*;

use crate::state::{Kickstarter, KickstarterState};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct EndPrivateRound<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::Unauthorized
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(ctx: Context<EndPrivateRound>) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;

    require!(kickstarter.state == KickstarterState::Live, ErrorCode::InvalidKickstarterState);
    require!(kickstarter.is_private_round_active, ErrorCode::InvalidKickstarterState);

    kickstarter.is_private_round_active = false;

    msg!("Private round ended");

    Ok(())
}