use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_token_metadata::{
    instructions::{
        CreateMetadataAccountV3Cpi, CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
    },
    types::DataV2,
};  

use crate::state::{Kickstarter, KickstarterState, PerformancePackage, PrivateFundState};
use crate::constants::{SEED_BASE_VAULT, SEED_QUOTE_VAULT, SEED_PRIVATE_STATE, MAX_PERFORMANCE_PACKAGES};

#[derive(Accounts)]
pub struct InitializeKickstarter<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Kickstarter::INIT_SPACE,
        seeds = [b"kickstarter", admin.key().as_ref(), base_mint.key().as_ref()],
        bump
    )]
    pub kickstarter: Box<Account<'info, Kickstarter>>,
    pub base_mint: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        token::mint = base_mint,
        token::authority = kickstarter,
        seeds = [SEED_BASE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        token::mint = quote_mint,
        token::authority = kickstarter,
        seeds = [SEED_QUOTE_VAULT.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub quote_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        space = 8 + PrivateFundState::INIT_SPACE,
        seeds = [SEED_PRIVATE_STATE.as_bytes(), kickstarter.key().as_ref()],
        bump
    )]
    pub private_state: Box<Account<'info, PrivateFundState>>,
    pub treasury: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>, //реальное списание будет в cpi вызове


    // pub mpl_program: Program<'info, mpl_token_metadata::programs::
    /// CHECK: verification in cpi
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    /// CHECK: checking ID in CPI call
    pub token_metadata_program: UncheckedAccount<'info>, //unchecked- можно не грузить бинарник в тестах
}

pub fn handler(
    ctx: Context<InitializeKickstarter>,
    minimum_raise_amount: u64,
    total_base_tokens_for_investors: u64,
    performance_pool_base_tokens: u64,
    seconds_for_launch: u32,
    monthly_team_spending_usdc: u64,
    package_unlock_delay_seconds: i64,
    token_name: String,
    token_symbol: String,
    _token_description: String, 
    token_image_url: String,
) -> Result<()> {
    let kickstarter = &mut ctx.accounts.kickstarter;
    
    kickstarter.kickstarter_authority = ctx.accounts.admin.key();
    kickstarter.state = KickstarterState::Initialized;
    kickstarter.base_mint = ctx.accounts.base_mint.key();
    kickstarter.quote_mint = ctx.accounts.quote_mint.key();
    kickstarter.base_vault = ctx.accounts.base_vault.key();
    kickstarter.quote_vault = ctx.accounts.quote_vault.key();
    kickstarter.treasury = ctx.accounts.treasury.key();
    kickstarter.minimum_raise_amount = minimum_raise_amount;
    kickstarter.hard_cap = u64::MAX;
    kickstarter.total_base_tokens_for_investors = total_base_tokens_for_investors;
    kickstarter.performance_pool_base_tokens = performance_pool_base_tokens;
    kickstarter.configured_performance_tokens = 0;
    kickstarter.performance_packages = [PerformancePackage::default(); MAX_PERFORMANCE_PACKAGES];
    kickstarter.seconds_for_launch = seconds_for_launch;
    kickstarter.total_committed_amount = 0;
    kickstarter.pda_bump = ctx.bumps.kickstarter;
    kickstarter.final_raise_amount = None;
    kickstarter.total_committed_at_completion = None;
    kickstarter.unix_timestamp_started = None;
    kickstarter.unix_timestamp_closed = None;
    kickstarter.monthly_team_spending_usdc = monthly_team_spending_usdc;
    kickstarter.package_unlock_delay_seconds = package_unlock_delay_seconds;
    kickstarter.calculated_liquidity_amount = None;
    kickstarter.initial_token_price = None;
    kickstarter.calculated_base_tokens_for_investors = None;
    kickstarter.calculated_base_tokens_for_liquidity = None;
    kickstarter.calculated_performance_pool_tokens = None;
    kickstarter.private_commitments_root = [0u8; 32];
    kickstarter.private_investor_count = 0;
    kickstarter.is_private_round_active = false;

    let private_state = &mut ctx.accounts.private_state;
    private_state.kickstarter = kickstarter.key();
    private_state.commitments_root = [0u8; 32];
    private_state.investor_count = 0;
    private_state.committed_amount = 0;
    private_state.bump = ctx.bumps.private_state;

    let admin_key = ctx.accounts.admin.key();
    let base_mint_key = ctx.accounts.base_mint.key();
    let seeds = [
        b"kickstarter", 
        admin_key.as_ref(),
        base_mint_key.as_ref(), 
        &[kickstarter.pda_bump]
    ];

    let metadata_info = ctx.accounts.metadata.to_account_info();
    let mint_info = ctx.accounts.base_mint.to_account_info();
    let authority_info = ctx.accounts.kickstarter.to_account_info();
    let payer_info = ctx.accounts.admin.to_account_info();
    let system_program_info = ctx.accounts.system_program.to_account_info();
    let rent_info = ctx.accounts.rent.to_account_info();

    let cpi_accounts = CreateMetadataAccountV3CpiAccounts {
        metadata: &metadata_info,
        mint: &mint_info,
        mint_authority: &authority_info,
        payer: &payer_info,
        update_authority: (&authority_info, true),
        system_program: &system_program_info,
        rent: Some(&rent_info),
    };

    let cpi_args = CreateMetadataAccountV3InstructionArgs {
        data: DataV2 {
            name: token_name.clone(),
            symbol: token_symbol.clone(),
            uri: token_image_url.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        is_mutable: true,
        collection_details: None,
    };

    CreateMetadataAccountV3Cpi::new(
        &ctx.accounts.token_metadata_program.to_account_info(), 
        cpi_accounts, 
        cpi_args
    ).invoke_signed(&[&seeds[..]])?;
    Ok(())
}

