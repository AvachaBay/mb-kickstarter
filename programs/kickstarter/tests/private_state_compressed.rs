use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use borsh::BorshDeserialize;
use light_client::indexer::TreeInfo;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use kickstarter::{
    ExistingPrivateStateCompressedIxData, LightCompressedAccountMetaReadOnly, LightCompressedProof,
    LightPackedAddressTreeInfo, LightPackedStateTreeInfo, LightValidityProof,
    PrivateStateCompressedData, PRIVATE_STATE_COMPRESSED_SEED,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_private_state_compressed() {
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("kickstarter", kickstarter::ID)]));
    config.log_light_protocol_events = true;
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_info = rpc.get_address_tree_v2();

    let kickstarter_key = Pubkey::new_unique();
    let commitments_root = [1u8; 32];
    let investor_count = 2u32;
    let committed_amount = 5_000_000u64;

    let (address, _) = light_sdk::address::v2::derive_address(
        &[PRIVATE_STATE_COMPRESSED_SEED, kickstarter_key.as_ref()],
        &address_tree_info.tree,
        &kickstarter::ID,
    );

    create_private_state_compressed(
        &mut rpc,
        &payer,
        &address,
        address_tree_info,
        kickstarter_key,
        commitments_root,
        investor_count,
        committed_amount,
    )
    .await
    .unwrap();

    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let data = &compressed_account.data.as_ref().unwrap().data;
    let account_data = PrivateStateCompressedData::deserialize(&mut &data[..]).unwrap();

    assert_eq!(account_data.kickstarter, kickstarter_key);
    assert_eq!(account_data.commitments_root, commitments_root);
    assert_eq!(account_data.investor_count, investor_count);
    assert_eq!(account_data.committed_amount, committed_amount);

    read_private_state_compressed(
        &mut rpc,
        &payer,
        &compressed_account,
        kickstarter_key,
        commitments_root,
        investor_count,
        committed_amount,
    )
    .await
    .unwrap();
}

async fn create_private_state_compressed<R>(
    rpc: &mut R,
    payer: &Keypair,
    address: &[u8; 32],
    address_tree_info: TreeInfo,
    kickstarter_key: Pubkey,
    commitments_root: [u8; 32],
    investor_count: u32,
    committed_amount: u64,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(kickstarter::ID);
    remaining_accounts.add_system_accounts_v2(config)?;

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: *address,
                tree: address_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;

    let packed_address_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .address_trees;
    let output_state_tree_index = rpc
        .get_random_state_tree_info()?
        .pack_output_tree_index(&mut remaining_accounts)?;

    let proof = LightValidityProof {
        proof: rpc_result.proof.0.map(|inner| LightCompressedProof {
            a: inner.a,
            b: inner.b,
            c: inner.c,
        }),
    };
    let address_tree_info = LightPackedAddressTreeInfo {
        address_merkle_tree_pubkey_index: packed_address_tree_accounts[0].address_merkle_tree_pubkey_index,
        address_queue_pubkey_index: packed_address_tree_accounts[0].address_queue_pubkey_index,
        root_index: packed_address_tree_accounts[0].root_index,
    };
    let instruction_data = kickstarter::instruction::CreatePrivateStateCompressed {
        proof,
        address_tree_info,
        output_state_tree_index,
        kickstarter: kickstarter_key,
        commitments_root,
        investor_count,
        committed_amount,
    };
    let accounts = kickstarter::accounts::PrivateStateCompressedAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();
    let instruction = Instruction {
        program_id: kickstarter::ID,
        accounts: [accounts.to_account_metas(None), remaining_accounts_metas].concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn read_private_state_compressed<R>(
    rpc: &mut R,
    payer: &Keypair,
    compressed_account: &light_client::indexer::CompressedAccount,
    kickstarter_key: Pubkey,
    commitments_root: [u8; 32],
    investor_count: u32,
    committed_amount: u64,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(kickstarter::ID);
    remaining_accounts.add_system_accounts_v2(config)?;

    let hash = compressed_account.hash;
    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let packed_state_tree_accounts = packed_tree_accounts.state_trees.unwrap();

    let account_meta = LightCompressedAccountMetaReadOnly {
        tree_info: LightPackedStateTreeInfo {
            root_index: packed_state_tree_accounts.packed_tree_infos[0].root_index,
            prove_by_index: packed_state_tree_accounts.packed_tree_infos[0].prove_by_index,
            merkle_tree_pubkey_index: packed_state_tree_accounts.packed_tree_infos[0].merkle_tree_pubkey_index,
            queue_pubkey_index: packed_state_tree_accounts.packed_tree_infos[0].queue_pubkey_index,
            leaf_index: packed_state_tree_accounts.packed_tree_infos[0].leaf_index,
        },
        address: compressed_account.address.unwrap(),
    };
    let proof = LightValidityProof {
        proof: rpc_result.proof.0.map(|inner| LightCompressedProof {
            a: inner.a,
            b: inner.b,
            c: inner.c,
        }),
    };

    let instruction_data = kickstarter::instruction::ReadPrivateStateCompressed {
        proof,
        existing_account: ExistingPrivateStateCompressedIxData {
            account_meta,
            kickstarter: kickstarter_key,
            commitments_root,
            investor_count,
            committed_amount,
        },
    };

    let accounts = kickstarter::accounts::PrivateStateCompressedAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();
    let instruction = Instruction {
        program_id: kickstarter::ID,
        accounts: [accounts.to_account_metas(None), remaining_accounts_metas].concat(),
        data: instruction_data.data(),
    };
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
