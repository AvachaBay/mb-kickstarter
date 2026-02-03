mod support;

use kickstarter::state::KickstarterState;
use support::{InitConfig, KickstarterAccounts, Signer, TestHarness, TestResult, to_anchor_pubkey};

#[test]
fn initialize_kickstarter_sets_initial_state() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let ix = accounts.initialize_ix(InitConfig::default());
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;

    assert_eq!(decoded.state, KickstarterState::Initialized);
    assert_eq!(
        decoded.kickstarter_authority,
        to_anchor_pubkey(&accounts.admin.pubkey())
    );

    Ok(())
}

#[test]
fn start_kickstarter_succeeds() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let ix_init = accounts.initialize_ix(InitConfig::default());
    harness.send(ix_init, &accounts.admin)?;

    let ix_start = accounts.start_ix();
    harness.send(ix_start, &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.hard_cap = u64::MAX;
    })?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.state, KickstarterState::Live);
    assert!(decoded.unix_timestamp_started.is_some());

    Ok(())
}

