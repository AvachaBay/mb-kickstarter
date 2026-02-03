pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::access_control::structs::Member;
use ephemeral_rollups_sdk::anchor::ephemeral;

pub use constants::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("DAqDXj2S1ipsGpe4eAi7AXBUT4MPd61DMDrq7QkvKUoY");

#[ephemeral]
#[program]
pub mod kickstarter {
    use super::*;

    pub fn initialize_kickstarter(
        ctx: Context<InitializeKickstarter>,
        minimum_raise_amount: u64,
        total_base_tokens_for_investors: u64,
        performance_pool_base_tokens: u64,
        seconds_for_launch: u32,
        monthly_team_spending_usdc: u64,
        package_unlock_delay_seconds: i64,
        token_name: String,
        token_symbol: String,
        token_description: String,
        token_image_url: String,
    ) -> Result<()> {
        initialize_kickstarter::handler(
            ctx,
            minimum_raise_amount,
            total_base_tokens_for_investors,
            performance_pool_base_tokens,
            seconds_for_launch,
            monthly_team_spending_usdc,
            package_unlock_delay_seconds,
            token_name,
            token_symbol,
            token_description,
            token_image_url,
        )
    }

    pub fn start_kickstarter(ctx: Context<StartKickstarter>) -> Result<()> {
        start_kickstarter::handler(ctx)
    }

    pub fn fund(ctx: Context<Fund>, amount: u64) -> Result<()> {
        fund::handler(ctx, amount)
    }

    pub fn fund_private(ctx: Context<FundPrivate>, amount: u64, salt: [u8; 32]) -> Result<()> {
        fund_private::handler(ctx, amount, salt)
    }

    pub fn finalize_private_round(
        ctx: Context<FinalizePrivateRound>,
        final_commitments_root: [u8; 32],
        attested_total_amount: u64,
        attestation_signature: [u8; 64],
    ) -> Result<()> {
        finalize_private_round::handler(ctx, final_commitments_root, attested_total_amount, attestation_signature)
    }

    pub fn private_claim(
        ctx: Context<PrivateClaim>,
        amount: u64,
        salt: [u8; 32]
    ) -> Result<()> {
        private_claim::handler(ctx, amount, salt)
    }

    pub fn private_claim_compressed(
        ctx: Context<PrivateClaimCompressed>,
        amount: u64,
        salt: [u8; 32]
    ) -> Result<()> {
        private_claim_compressed::handler(ctx, amount, salt)
    }

    pub fn private_refund(
        ctx: Context<PrivateRefund>,
        amount: u64,
        salt: [u8; 32]
    ) -> Result<()> {
        private_refund::handler(ctx, amount, salt)
    }

    pub fn start_private_round(ctx: Context<StartPrivateRound>) -> Result<()> {
        start_private_round::handler(ctx)
    }

    pub fn end_private_round(ctx: Context<EndPrivateRound>) -> Result<()> {
        end_private_round::handler(ctx)
    }

    pub fn close_kickstarter(ctx: Context<CloseKickstarter>) -> Result<()> {
        close_kickstarter::handler(ctx)
    }

    pub fn complete_kickstarter(
        ctx: Context<CompleteKickstarter>,
        final_raise_amount: u64,
    ) -> Result<()> {
        complete_kickstarter::handler(ctx, final_raise_amount)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        claim::handler(ctx)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        refund::handler(ctx)
    }

    pub fn configure_performance_package(
        ctx: Context<ConfigurePerformancePackage>,
        index: u8,
        multiplier: u8,
        allocation: u64,
    ) -> Result<()> {
        configure_performance_package::handler(ctx, index, multiplier, allocation)
    }

    pub fn unlock_performance_package(
        ctx: Context<UnlockPerformancePackage>,
        index: u8,
        current_price: u64,
    ) -> Result<()> {
        unlock_performance_package::handler(ctx, index, current_price)
    }

    pub fn claim_performance_package(
        ctx: Context<ClaimPerformancePackage>,
        index: u8,
    ) -> Result<()> {
        claim_performance_package::handler(ctx, index)
    }

    pub fn set_minimum_raise(
        ctx: Context<SetMinimumRaise>,
        new_minimum_raise: u64,
    ) -> Result<()> {
        set_minimum_raise::handler(ctx, new_minimum_raise)
    }

    pub fn stake_from_treasury(
        ctx: Context<StakeFromTreasury>,
        amount: u64,
    ) -> Result<()> {
        stake_from_treasury::handler(ctx, amount)
    }

    pub fn create_permission(
        ctx: Context<CreatePermission>,
        account_type: PermissionedAccountType,
        members: Vec<Member>,
    ) -> Result<()> {
        permissions::create_permission(ctx, account_type, members)
    }

    pub fn delegate_pda(ctx: Context<DelegatePda>, account_type: PermissionedAccountType) -> Result<()> {
        permissions::delegate_pda(ctx, account_type)
    }

    pub fn undelegate_private_state(ctx: Context<UndelegatePrivateState>) -> Result<()> {
        permissions::undelegate_private_state(ctx)
    }

    pub fn create_private_state_compressed<'info>(
        ctx: Context<'_, '_, '_, 'info, PrivateStateCompressedAccounts<'info>>,
        proof: LightValidityProof,
        address_tree_info: LightPackedAddressTreeInfo,
        output_state_tree_index: u8,
        kickstarter: Pubkey,
        commitments_root: [u8; 32],
        investor_count: u32,
        committed_amount: u64,
    ) -> Result<()> {
        private_state_compressed::create_private_state_compressed(
            ctx,
            proof,
            address_tree_info,
            output_state_tree_index,
            kickstarter,
            commitments_root,
            investor_count,
            committed_amount,
        )
    }

    pub fn read_private_state_compressed<'info>(
        ctx: Context<'_, '_, '_, 'info, PrivateStateCompressedAccounts<'info>>,
        proof: LightValidityProof,
        existing_account: ExistingPrivateStateCompressedIxData,
    ) -> Result<()> {
        private_state_compressed::read_private_state_compressed(ctx, proof, existing_account)
    }
}
