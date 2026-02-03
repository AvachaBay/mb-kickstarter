use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};

use crate::events::RefundEvent;
use crate::state::{FunderPosition, Kickstarter, KickstarterState};
use crate::error::ErrorCode;

use crate::constants::SEED_QUOTE_VAULT;

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        mut,
        seeds = [b"funder_position", kickstarter.key().as_ref(), user.key().as_ref()],
        bump = funder_position.bump
    )]
    pub funder_position: Account<'info, FunderPosition>,
    #[account(
        mut,
        address = kickstarter.quote_vault,
        seeds = [SEED_QUOTE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub quote_vault: InterfaceAccount<'info, SplTokenAccount>,
    #[account(mut)]
    pub user_quote_account: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Refund>) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let funder_position = &mut ctx.accounts.funder_position;
    
    let total_refundable = match kickstarter.state {
        KickstarterState::Refunding => funder_position.committed_amount,
        KickstarterState::Complete => {
            let final_raise = kickstarter
                .final_raise_amount
                .ok_or(ErrorCode::FinalRaiseAmountMissing)?;
            let total_committed_snapshot = kickstarter
                .total_committed_at_completion
                .ok_or(ErrorCode::CommittedSnapshotMissing)?;
            require!(total_committed_snapshot > 0, ErrorCode::CommittedSnapshotMissing);
            let accepted_u128 = (funder_position.committed_amount as u128)
                .checked_mul(final_raise as u128)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(total_committed_snapshot as u128)
                .ok_or(ErrorCode::MathOverflow)?;
            let accepted = u64::try_from(accepted_u128).map_err(|_| ErrorCode::MathOverflow)?;
            funder_position
                .committed_amount
                .checked_sub(accepted)
                .ok_or(ErrorCode::MathOverflow)?
        }
        _ => return err!(ErrorCode::InvalidKickstarterState),
    };

    let refund_amount = total_refundable
        .checked_sub(funder_position.claimed_refund)
        .unwrap();
    if refund_amount > 0 {
        let seeds = &[
            b"kickstarter", 
            kickstarter.kickstarter_authority.as_ref(),
            kickstarter.base_mint.as_ref(), 
            &[kickstarter.pda_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.quote_vault.to_account_info(),
            to: ctx.accounts.user_quote_account.to_account_info(),
            authority: kickstarter.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts, 
            signer
        );
        token::transfer(cpi_ctx, refund_amount)?;

        funder_position.claimed_refund = funder_position.claimed_refund.checked_add(refund_amount).unwrap();
        emit!(RefundEvent {
            kickstarter: kickstarter.key(),
            user: ctx.accounts.user.key(),
            amount: refund_amount,
            state: kickstarter.state,
        });
    }

    Ok(())
}
