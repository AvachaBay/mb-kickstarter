mod support;

use kickstarter::state::KickstarterState;
use solana_keypair::Keypair;
use support::{
    finalize_private_round_ix, fund_private_ix, InitConfig, KickstarterAccounts,
    Signer, TestHarness, TestResult,
};

#[test]
fn fund_private_updates_commitments_and_count() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;
    let user = Keypair::new();
    harness.airdrop(&user.pubkey(), 10_000_000_000)?;

    let config = InitConfig::default();
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.hard_cap = u64::MAX;
    })?;

    let salt = [1u8; 32]; // Simple salt for testing
    let ix_fund_private = fund_private_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.private_state,
        5_000_000,
        salt,
    );
    harness.send(ix_fund_private, &user)?;

    let private_state = harness.private_state(&accounts.private_state)?;
    assert_eq!(private_state.committed_amount, 5_000_000);

    assert_eq!(private_state.investor_count, 1);

    // Check that private_commitments_root hasbeen updated (should not be all zeros)
    assert_ne!(private_state.commitments_root, [0u8; 32]);

    Ok(())
}

#[test]
fn finalize_private_round_verifies_commitments() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;
    let user = Keypair::new();
    harness.airdrop(&user.pubkey(), 10_000_000_000)?;

    let config = InitConfig::default();
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.hard_cap = u64::MAX;
    })?;

    let salt = [1u8; 32];
    let ix_fund_private = fund_private_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.private_state,
        5_000_000,
        salt,
    );
    harness.send(ix_fund_private, &user)?;

    let private_state = harness.private_state(&accounts.private_state)?;

    let ix_finalize = finalize_private_round_ix(
        &accounts.admin.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.private_state,
        private_state.commitments_root,
        private_state.committed_amount,
        [0u8; 64], 
    );
    harness.send(ix_finalize, &accounts.admin)?;

    Ok(())
}