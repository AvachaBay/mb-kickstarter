use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Mint, MintTo};
use anchor_spl::token_interface::TokenAccount;

use crate::{
    events::CompleteEvent,
    state::{Kickstarter, KickstarterState},
};
use crate::error::ErrorCode;
use crate::constants::{
    BPS_DENOMINATOR, 
    LIQUIDITY_BPS,
    BASE_TOKENS_FOR_INVESTORS_BPS,
    BASE_TOKENS_FOR_LIQUIDITY_BPS,
};

#[derive(Accounts)]
pub struct CompleteKickstarter<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        constraint = kickstarter.kickstarter_authority == admin.key() @ ErrorCode::InvalidAdmin
    )]
    pub kickstarter: Box<Account<'info, Kickstarter>>,

    #[account(
        mut,
        address = kickstarter.quote_vault
    )]
    pub quote_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        constraint = treasury_token_account.owner == kickstarter.treasury @ ErrorCode::InvalidTreasuryAccountOwner,
        constraint = treasury_token_account.mint == kickstarter.quote_mint @ ErrorCode::InvalidQuoteMint,
    )]
    pub treasury_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = liquidity_token_account.mint == kickstarter.quote_mint @ ErrorCode::InvalidQuoteMint,
    )]
    pub liquidity_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        address = kickstarter.base_vault
    )]
    pub base_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        constraint = liquidity_base_token_account.mint == kickstarter.base_mint @ ErrorCode::InvalidBaseMint,
    )]
    pub liquidity_base_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        address = kickstarter.base_mint
    )]
    pub base_mint: Box<Account<'info, Mint>>,
    
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<CompleteKickstarter>, final_raise_amount: u64) -> Result<()> {
    require!(ctx.accounts.kickstarter.state == KickstarterState::Live, ErrorCode::InvalidKickstarterState);

    let current_time = Clock::get()?.unix_timestamp;
        if let Some(closed_time) = ctx.accounts.kickstarter.unix_timestamp_closed {
            if current_time < closed_time {
            return err!(ErrorCode::TooEarlyToCompleteKickstarter);
            }
        }

    let mut liquidity_amount: u64 = 0;
    let mut treasury_amount: u64 = 0;
    let mut final_raise_for_event: Option<u64> = None;

    if ctx.accounts.kickstarter.total_committed_amount < ctx.accounts.kickstarter.minimum_raise_amount {
        ctx.accounts.kickstarter.state = KickstarterState::Refunding; //галя, у нас возврат
        ctx.accounts.kickstarter.final_raise_amount = None;
        ctx.accounts.kickstarter.total_committed_at_completion = None;
    } else {
        require!(final_raise_amount > 0, ErrorCode::InvalidFinalRaiseAmount);
        require!(
            final_raise_amount >= ctx.accounts.kickstarter.minimum_raise_amount,
            ErrorCode::InvalidFinalRaiseAmount
        );
        require!(
            final_raise_amount <= ctx.accounts.kickstarter.total_committed_amount,
            ErrorCode::FinalAmountExceedsTotalCommitted
        );

        let admin_key = ctx.accounts.kickstarter.kickstarter_authority;
        let base_mint_key = ctx.accounts.kickstarter.base_mint;
        let bump = ctx.accounts.kickstarter.pda_bump;
        let total_committed = ctx.accounts.kickstarter.total_committed_amount;
        
        let base_tokens_for_investors_fixed = ctx.accounts.kickstarter.total_base_tokens_for_investors;
        let performance_pool_fixed = ctx.accounts.kickstarter.performance_pool_base_tokens;
        
        let calculated_base_tokens_for_liquidity_u128 = (base_tokens_for_investors_fixed as u128)
            .checked_mul(BASE_TOKENS_FOR_LIQUIDITY_BPS as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(BASE_TOKENS_FOR_INVESTORS_BPS as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        let calculated_base_tokens_for_liquidity = u64::try_from(calculated_base_tokens_for_liquidity_u128)
            .map_err(|_| ErrorCode::MathOverflow)?;
        
        ctx.accounts.kickstarter.state = KickstarterState::Complete;
        ctx.accounts.kickstarter.final_raise_amount = Some(final_raise_amount);
        ctx.accounts.kickstarter.total_committed_at_completion = Some(total_committed);
        ctx.accounts.kickstarter.calculated_base_tokens_for_investors = Some(base_tokens_for_investors_fixed);
        ctx.accounts.kickstarter.calculated_base_tokens_for_liquidity = Some(calculated_base_tokens_for_liquidity);
        ctx.accounts.kickstarter.calculated_performance_pool_tokens = Some(performance_pool_fixed);
        final_raise_for_event = Some(final_raise_amount);
        
        let initial_price_u128 = (final_raise_amount as u128)
            .checked_mul(1_000_000_000_000u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(base_tokens_for_investors_fixed as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        let initial_price = u64::try_from(initial_price_u128).map_err(|_| ErrorCode::MathOverflow)?;
        ctx.accounts.kickstarter.initial_token_price = Some(initial_price);
        
        let seeds = &[
            b"kickstarter",
            admin_key.as_ref(),
            base_mint_key.as_ref(),
            &[bump]
        ];
        let signer = &[&seeds[..]];

        let liquidity_amount_u128 = (final_raise_amount as u128)
            .checked_mul(LIQUIDITY_BPS as u128)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(BPS_DENOMINATOR as u128)
            .ok_or(ErrorCode::MathOverflow)?;
        liquidity_amount = u64::try_from(liquidity_amount_u128).map_err(|_| ErrorCode::MathOverflow)?;
        
        let monthly_spending = ctx.accounts.kickstarter.monthly_team_spending_usdc;
        
        let remaining_after_liquidity = final_raise_amount
            .checked_sub(liquidity_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        
        require!(
            remaining_after_liquidity >= monthly_spending,
            ErrorCode::MathOverflow
        );
        
        treasury_amount = remaining_after_liquidity
            .checked_sub(monthly_spending)
            .ok_or(ErrorCode::MathOverflow)?;

        if liquidity_amount > 0 {
            let cpi_ctx_liquidity = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.quote_vault.to_account_info(),
                    to: ctx.accounts.liquidity_token_account.to_account_info(),
                    authority: ctx.accounts.kickstarter.to_account_info(),
                },
                signer,
            );
            token::transfer(cpi_ctx_liquidity, liquidity_amount)?;
        }

        let total_base_to_mint = base_tokens_for_investors_fixed
            .checked_add(calculated_base_tokens_for_liquidity)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_add(performance_pool_fixed)
            .ok_or(ErrorCode::MathOverflow)?;

        if total_base_to_mint > 0 {
            let cpi_ctx_mint_to_vault = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.base_mint.to_account_info(),
                    to: ctx.accounts.base_vault.to_account_info(),
                    authority: ctx.accounts.kickstarter.to_account_info(),
                },
                signer,
            );
            token::mint_to(cpi_ctx_mint_to_vault, total_base_to_mint)?;
        }

        if calculated_base_tokens_for_liquidity > 0 {
            let cpi_ctx_base_liquidity = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.base_vault.to_account_info(),
                    to: ctx.accounts.liquidity_base_token_account.to_account_info(),
                    authority: ctx.accounts.kickstarter.to_account_info(),
                },
                signer,
            );
            token::transfer(cpi_ctx_base_liquidity, calculated_base_tokens_for_liquidity)?;
        }

        if monthly_spending > 0 {
            let cpi_ctx_monthly_spending = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.quote_vault.to_account_info(),
                    to: ctx.accounts.treasury_token_account.to_account_info(),
                    authority: ctx.accounts.kickstarter.to_account_info(),
                },
                signer,
            );
            token::transfer(cpi_ctx_monthly_spending, monthly_spending)?;
        }

        if treasury_amount > 0 {
            let cpi_ctx_treasury = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                token::Transfer {
                    from: ctx.accounts.quote_vault.to_account_info(),
                    to: ctx.accounts.treasury_token_account.to_account_info(), 
                    authority: ctx.accounts.kickstarter.to_account_info(),
                },
                signer
            );
            
            token::transfer(cpi_ctx_treasury, treasury_amount)?;
        }
        
        ctx.accounts.kickstarter.calculated_liquidity_amount = Some(liquidity_amount);
    }

    ctx.accounts.kickstarter.unix_timestamp_closed = Some(current_time);

    emit!(CompleteEvent {
        kickstarter: ctx.accounts.kickstarter.key(),
        state: ctx.accounts.kickstarter.state,
        final_raise_amount: final_raise_for_event,
        liquidity_amount,
        treasury_amount,
    });

    Ok(())
}
