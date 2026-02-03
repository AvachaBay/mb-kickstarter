#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::cpi::{v2::LightSystemProgramCpi, InvokeLightSystemProgram, LightCpiInstruction};
use light_sdk::{
    account::LightAccount,
    address::v2::derive_address,
    cpi::{v2::CpiAccounts, CpiSigner},
    derive_light_cpi_signer,
    instruction::{
        account_meta::CompressedAccountMetaReadOnly, CompressedProof, PackedAddressTreeInfo,
        PackedStateTreeInfo, ValidityProof,
    },
    LightDiscriminator,
};
use light_sdk::constants::ADDRESS_TREE_V2;

pub const PRIVATE_STATE_COMPRESSED_SEED: &[u8] = b"private_state_compressed";

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("DAqDXj2S1ipsGpe4eAi7AXBUT4MPd61DMDrq7QkvKUoY"); 

#[derive(Accounts)]
pub struct PrivateStateCompressedAccounts<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize, LightDiscriminator)]
pub struct PrivateStateCompressedData {
    pub kickstarter: Pubkey,
    pub commitments_root: [u8; 32],
    pub investor_count: u32,
    pub committed_amount: u64,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightPackedStateTreeInfo {
    pub root_index: u16,
    pub prove_by_index: bool,
    pub merkle_tree_pubkey_index: u8,
    pub queue_pubkey_index: u8,
    pub leaf_index: u32,
}

impl From<LightPackedStateTreeInfo> for PackedStateTreeInfo {
    fn from(value: LightPackedStateTreeInfo) -> Self {
        Self {
            root_index: value.root_index,
            prove_by_index: value.prove_by_index,
            merkle_tree_pubkey_index: value.merkle_tree_pubkey_index,
            queue_pubkey_index: value.queue_pubkey_index,
            leaf_index: value.leaf_index,
        }
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightCompressedAccountMetaReadOnly {
    pub tree_info: LightPackedStateTreeInfo,
    pub address: [u8; 32],
}

impl From<LightCompressedAccountMetaReadOnly> for CompressedAccountMetaReadOnly {
    fn from(value: LightCompressedAccountMetaReadOnly) -> Self {
        Self {
            tree_info: value.tree_info.into(),
            address: value.address,
        }
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightPackedAddressTreeInfo {
    pub address_merkle_tree_pubkey_index: u8,
    pub address_queue_pubkey_index: u8,
    pub root_index: u16,
}

impl From<LightPackedAddressTreeInfo> for PackedAddressTreeInfo {
    fn from(value: LightPackedAddressTreeInfo) -> Self {
        Self {
            address_merkle_tree_pubkey_index: value.address_merkle_tree_pubkey_index,
            address_queue_pubkey_index: value.address_queue_pubkey_index,
            root_index: value.root_index,
        }
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightCompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightValidityProof {
    pub proof: Option<LightCompressedProof>,
}

impl From<LightValidityProof> for ValidityProof {
    fn from(value: LightValidityProof) -> Self {
        let proof = value.proof.map(|inner| CompressedProof {
            a: inner.a,
            b: inner.b,
            c: inner.c,
        });
        ValidityProof(proof)
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct ExistingPrivateStateCompressedIxData {
    pub account_meta: LightCompressedAccountMetaReadOnly,
    pub kickstarter: Pubkey,
    pub commitments_root: [u8; 32],
    pub investor_count: u32,
    pub committed_amount: u64,
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
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        LIGHT_CPI_SIGNER,
    );

    let address_tree_info: PackedAddressTreeInfo = address_tree_info.into();
    let proof: ValidityProof = proof.into();
    let address_tree_pubkey = address_tree_info
        .get_tree_pubkey(&light_cpi_accounts)
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;

    if address_tree_pubkey.to_bytes() != ADDRESS_TREE_V2 {
        return Err(ProgramError::InvalidAccountData.into());
    }

    let (address, address_seed) = derive_address(
        &[PRIVATE_STATE_COMPRESSED_SEED, kickstarter.as_ref()],
        &address_tree_pubkey,
        &crate::ID,
    );

    let mut data_account = LightAccount::<PrivateStateCompressedData>::new_init(
        &crate::ID,
        Some(address),
        output_state_tree_index,
    );

    data_account.kickstarter = kickstarter;
    data_account.commitments_root = commitments_root;
    data_account.investor_count = investor_count;
    data_account.committed_amount = committed_amount;

    let new_address_params =
        address_tree_info.into_new_address_params_assigned_packed(address_seed, Some(0));

    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
        .with_light_account(data_account)?
        .with_new_addresses(&[new_address_params])
        .invoke(light_cpi_accounts)?;

    Ok(())
}

pub fn read_private_state_compressed<'info>(
    ctx: Context<'_, '_, '_, 'info, PrivateStateCompressedAccounts<'info>>,
    proof: LightValidityProof,
    existing_account: ExistingPrivateStateCompressedIxData,
) -> Result<()> {
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        LIGHT_CPI_SIGNER,
    );

    let read_data_account = PrivateStateCompressedData {
        kickstarter: existing_account.kickstarter,
        commitments_root: existing_account.commitments_root,
        investor_count: existing_account.investor_count,
        committed_amount: existing_account.committed_amount,
    };

    let read_only_account = LightAccount::<PrivateStateCompressedData>::new_read_only(
        &crate::ID,
        &existing_account.account_meta.into(),
        read_data_account,
        light_cpi_accounts.tree_pubkeys().unwrap().as_slice(),
    )?;

    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof.into())
        .with_light_account(read_only_account)?
        .invoke(light_cpi_accounts)?;

    Ok(())
}
