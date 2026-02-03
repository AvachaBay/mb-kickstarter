use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, Transfer},
    token_interface::TokenAccount as SplTokenAccount,
};

use crate::events::ClaimEvent;
use crate::state::{FunderPosition, Kickstarter, KickstarterState};
use crate::error::ErrorCode;

use crate::constants::SEED_BASE_VAULT;

#[derive(Accounts)]
pub struct Claim<'info> {
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
        address = kickstarter.base_vault,
        seeds = [SEED_BASE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub base_vault: InterfaceAccount<'info, SplTokenAccount>,
    #[account(mut)]
    pub user_base_account: InterfaceAccount<'info, SplTokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Claim>) -> Result<()> {
    let kickstarter = &ctx.accounts.kickstarter;
    let funder_position = &mut ctx.accounts.funder_position;

    require!(kickstarter.state == KickstarterState::Complete, ErrorCode::InvalidKickstarterState);

    

    // 1 - 1 ratio rn -> switch to "honest" distribution
    //let claim_amount = funder_position.committed_amount.checked_sub(funder_position.already_claimed_base).unwrap();
        
    //switching к расчету доли of user которую он внес
    let total_committed_snapshot = kickstarter
        .total_committed_at_completion
        .ok_or(ErrorCode::CommittedSnapshotMissing)?;
    require!(total_committed_snapshot > 0, ErrorCode::CommittedSnapshotMissing);

    let base_tokens_to_user_u128 = 
        (funder_position.committed_amount as u128)
        .checked_mul(kickstarter.total_base_tokens_for_investors as u128).unwrap()
        .checked_div(total_committed_snapshot as u128).unwrap();
    let base_tokens_to_user_u64 = u64::try_from(base_tokens_to_user_u128)
        .unwrap(); //add error

    let tokens_to_claim = (base_tokens_to_user_u64)
            .checked_sub(funder_position.already_claimed_base).unwrap(); //left to claim

    if base_tokens_to_user_u64 > 0 {

        let seeds = &[
            b"kickstarter", 
            kickstarter.kickstarter_authority.as_ref(),
            kickstarter.base_mint.as_ref(), 
            &[kickstarter.pda_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.base_vault.to_account_info(),
            to: ctx.accounts.user_base_account.to_account_info(),
            authority: kickstarter.to_account_info(),
        };
        let cpi_ctx: CpiContext<'_, '_, '_, '_, Transfer<'_>> = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts, 
            signer
        );
        token::transfer(cpi_ctx, base_tokens_to_user_u64)?; //claim_amount

        //update сколько он забрал
        funder_position.already_claimed_base = funder_position.already_claimed_base
            .checked_add(tokens_to_claim).unwrap();

        emit!(ClaimEvent {
            kickstarter: kickstarter.key(),
            user: ctx.accounts.user.key(),
            amount: tokens_to_claim,
            total_claimed: funder_position.already_claimed_base,
        });
    }

    Ok(())
}
