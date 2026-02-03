use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::access_control::instructions::CreatePermissionCpiBuilder;
use ephemeral_rollups_sdk::access_control::structs::{Member, MembersArgs};
use ephemeral_rollups_sdk::anchor::{commit, delegate};
use ephemeral_rollups_sdk::consts::PERMISSION_PROGRAM_ID;
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

use crate::constants::{SEED_KICKSTARTER, SEED_PRIVATE_STATE, SEED_QUOTE_VAULT};
use crate::state::{Kickstarter, PrivateFundState};

pub fn create_permission(
    ctx: Context<CreatePermission>,
    account_type: PermissionedAccountType,
    members: Vec<Member>,
) -> Result<()> {
    let seed_data = derive_seeds_from_account_type(&account_type);
    let seed_refs: Vec<&[u8]> = seed_data.iter().map(|s| s.as_slice()).collect();
    let (derived, bump) = Pubkey::find_program_address(&seed_refs, &crate::ID);
    require_keys_eq!(derived, ctx.accounts.permissioned_account.key());
    let mut signer_seeds = seed_data;
    signer_seeds.push(vec![bump]);
    let signer_refs: Vec<&[u8]> = signer_seeds.iter().map(|s| s.as_slice()).collect();

    CreatePermissionCpiBuilder::new(&ctx.accounts.permission_program.to_account_info())
        .permissioned_account(&ctx.accounts.permissioned_account.to_account_info())
        .permission(&ctx.accounts.permission)
        .payer(&ctx.accounts.payer)
        .system_program(&ctx.accounts.system_program.to_account_info())
        .args(MembersArgs { members: Some(members) })
        .invoke_signed(&[signer_refs.as_slice()])?;
    Ok(())
}

pub fn delegate_pda(ctx: Context<DelegatePda>, account_type: PermissionedAccountType) -> Result<()> {
    let seed_data = derive_seeds_from_account_type(&account_type);
    let seed_refs: Vec<&[u8]> = seed_data.iter().map(|s| s.as_slice()).collect();
    let validator = ctx.accounts.validator.as_ref().map(|v| v.key());
    ctx.accounts.delegate_pda(
        &ctx.accounts.payer,
        &seed_refs,
        DelegateConfig {
            validator,
            commit_frequency_ms: 0,
            ..Default::default()
        },
    )?;
    Ok(())
}

pub fn undelegate_private_state(ctx: Context<UndelegatePrivateState>) -> Result<()> {
    commit_and_undelegate_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.private_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct CreatePermission<'info> {
    /// CHECK: checked
    pub permissioned_account: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked
    pub permission: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: checked
    pub payer: Signer<'info>,
    #[account(address = PERMISSION_PROGRAM_ID)]
    /// CHECK: checked
    pub permission_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegatePda<'info> {
    /// CHECK: checked
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
    /// CHECK: checked
    pub payer: Signer<'info>,
    pub validator: Option<AccountInfo<'info>>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegatePrivateState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub kickstarter: Account<'info, Kickstarter>,
    #[account(
        mut,
        seeds = [SEED_PRIVATE_STATE.as_bytes(), kickstarter.key().as_ref()],
        bump,
        has_one = kickstarter
    )]
    pub private_state: Account<'info, PrivateFundState>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum PermissionedAccountType {
    Kickstarter { admin: Pubkey, base_mint: Pubkey },
    QuoteVault { kickstarter: Pubkey },
    PrivateState { kickstarter: Pubkey },
}

fn derive_seeds_from_account_type(account_type: &PermissionedAccountType) -> Vec<Vec<u8>> {
    match account_type {
        PermissionedAccountType::Kickstarter { admin, base_mint } => {
            vec![SEED_KICKSTARTER.as_bytes().to_vec(), admin.to_bytes().to_vec(), base_mint.to_bytes().to_vec()]
        }
        PermissionedAccountType::QuoteVault { kickstarter } => {
            vec![SEED_QUOTE_VAULT.as_bytes().to_vec(), kickstarter.to_bytes().to_vec()]
        }
        PermissionedAccountType::PrivateState { kickstarter } => {
            vec![SEED_PRIVATE_STATE.as_bytes().to_vec(), kickstarter.to_bytes().to_vec()]
        }
    }
}
