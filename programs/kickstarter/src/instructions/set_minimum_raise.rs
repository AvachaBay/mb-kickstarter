use anchor_lang::prelude::*;

use crate::{
    events::MinimumRaiseUpdatedEvent,
    error::ErrorCode,
    state::{Kickstarter, KickstarterState},
};

#[derive(Accounts)]
pub struct SetMinimumRaise<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin
    )]
    pub kickstarter: Account<'info, Kickstarter>,
}

pub fn handler(ctx: Context<SetMinimumRaise>, new_minimum: u64) -> Result<()> {
    require!(new_minimum > 0, ErrorCode::InvalidMinimumRaiseAmount);

    let kickstarter = &mut ctx.accounts.kickstarter;
    require!(
        matches!(
            kickstarter.state,
            KickstarterState::Initialized | KickstarterState::Live
        ),
        ErrorCode::InvalidKickstarterState
    );

    kickstarter.minimum_raise_amount = new_minimum;
    kickstarter.hard_cap = u64::MAX;

    emit!(MinimumRaiseUpdatedEvent {
        kickstarter: kickstarter.key(),
        new_minimum,
    });

    Ok(())
}

