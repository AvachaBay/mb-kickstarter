use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};

use crate::events::FundEvent;
use crate::state::{FunderPosition, Kickstarter, KickstarterState};
use crate::error::ErrorCode;

use crate::constants::SEED_QUOTE_VAULT;

#[derive(Accounts)]
pub struct Fund<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    #[account(mut)]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        init_if_needed,
        payer = funder,
        space = 8 + FunderPosition::INIT_SPACE,
        seeds = [b"funder_position", kickstarter.key().as_ref(), funder.key().as_ref()],
        bump
    )]
    pub funder_position: Account<'info, FunderPosition>,
    #[account(mut)]
    pub funder_quote_account: InterfaceAccount<'info, SplTokenAccount>,
    #[account(
        mut,
        address = kickstarter.quote_vault,
        seeds = [SEED_QUOTE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub quote_vault: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Fund>, amount: u64) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;
    let funder_position = &mut ctx.accounts.funder_position;
    
    require!(kickstarter.state == KickstarterState::Live, ErrorCode::InvalidKickstarterState);

    if let Some(closed_time) = kickstarter.unix_timestamp_closed {
        if Clock::get()?.unix_timestamp < closed_time {
            return err!(ErrorCode::TooEarlyToCompleteKickstarter);
        }
    }
    if kickstarter.total_committed_amount.checked_add(amount) > Some(kickstarter.hard_cap) {
        return err!(ErrorCode::OverHardcapLimit);
    }
    //TODO make partial refund, it just resigns the current depo for now


    let cpi_accounts = Transfer {
        from: ctx.accounts.funder_quote_account.to_account_info(),
        to: ctx.accounts.quote_vault.to_account_info(),
        authority: ctx.accounts.funder.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;
    
    kickstarter.total_committed_amount = kickstarter.total_committed_amount.checked_add(amount).unwrap();
    
    if funder_position.kickstarter == Pubkey::default() {
        funder_position.kickstarter = kickstarter.key();
        funder_position.user = ctx.accounts.funder.key();
        funder_position.bump = ctx.bumps.funder_position;
    }

    funder_position.committed_amount = funder_position.committed_amount.checked_add(amount).unwrap();

    emit!(FundEvent {
        kickstarter: kickstarter.key(),
        funder: ctx.accounts.funder.key(),
        amount,
        total_committed: kickstarter.total_committed_amount,
    });

    Ok(())
}
