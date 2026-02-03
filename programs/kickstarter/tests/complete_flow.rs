mod support;

use kickstarter::state::KickstarterState;
use support::{InitConfig, KickstarterAccounts, TestHarness, TestResult};

#[test]
fn complete_kickstarter_success_path() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    const FINAL_RAISE_AMOUNT: u64 = 1_000_000;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.total_committed_amount = FINAL_RAISE_AMOUNT;
    })?;

    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        FINAL_RAISE_AMOUNT,
    )?;

    let ix = accounts.complete_ix(FINAL_RAISE_AMOUNT);
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.state, KickstarterState::Complete);
    assert!(decoded.calculated_liquidity_amount.is_some());

    Ok(())
}

#[test]
fn complete_kickstarter_refund_path() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.total_committed_amount = 100_000;
    })?;

    let ix = accounts.complete_ix(0);
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.state, KickstarterState::Refunding);

    Ok(())
}

#[test]
fn monthly_team_spending_usdc_is_stored() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    const MONTHLY_SPENDING: u64 = 50_000_000;
    let config = InitConfig {
        minimum_raise_amount: 500_000,
        monthly_team_spending_usdc: MONTHLY_SPENDING,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.monthly_team_spending_usdc, MONTHLY_SPENDING);

    Ok(())
}

#[test]
fn calculated_liquidity_amount_is_set_after_complete() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    const FINAL_RAISE_AMOUNT: u64 = 1_000_000;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.total_committed_amount = FINAL_RAISE_AMOUNT;
    })?;

    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        FINAL_RAISE_AMOUNT,
    )?;

    let ix = accounts.complete_ix(FINAL_RAISE_AMOUNT);
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert!(decoded.calculated_liquidity_amount.is_some());
    let liquidity = decoded.calculated_liquidity_amount.unwrap();
    assert_eq!(liquidity, FINAL_RAISE_AMOUNT * 2000 / 10000);

    Ok(())
}

