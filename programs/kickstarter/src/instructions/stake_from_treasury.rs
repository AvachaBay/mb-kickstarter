use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer};
use anchor_spl::token_interface::TokenAccount;

use crate::{
    events::StakeFromTreasuryEvent,
    state::Kickstarter,
    error::ErrorCode,
};

#[derive(Accounts)]
pub struct StakeFromTreasury<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin
    )]
    pub kickstarter: Account<'info, Kickstarter>,
    
    #[account(
        mut,
        constraint = treasury_token_account.owner == kickstarter.treasury @ ErrorCode::InvalidTreasuryAccountOwner,
        constraint = treasury_token_account.mint == kickstarter.quote_mint @ ErrorCode::InvalidQuoteMint,
    )]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,
    
    /// CHECK: Staking acc
    #[account(mut)]
    pub staking_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<StakeFromTreasury>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidFinalRaiseAmount);
    
    let kickstarter = &ctx.accounts.kickstarter;
    let treasury_token_account = &ctx.accounts.treasury_token_account;
    
    require!(
        treasury_token_account.amount >= amount,
        ErrorCode::MathOverflow
    );
    
    let cpi_accounts = Transfer {
        from: treasury_token_account.to_account_info(),
        to: ctx.accounts.staking_account.to_account_info(),
        authority: ctx.accounts.admin.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    token::transfer(cpi_ctx, amount)?;
    
    emit!(StakeFromTreasuryEvent {
        kickstarter: kickstarter.key(),
        admin: ctx.accounts.admin.key(),
        amount,
        staking_account: ctx.accounts.staking_account.key(),
    });
    
    Ok(())
}

