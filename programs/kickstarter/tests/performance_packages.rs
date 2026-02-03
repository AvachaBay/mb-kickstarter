mod support;

use support::{InitConfig, KickstarterAccounts, Pubkey, Signer, TestHarness, TestResult};

#[test]
fn performance_packages_flow() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    const PERFORMANCE_POOL: u64 = 2_000_000_000_000;
    const PACKAGE_ALLOCATION: u64 = 1_500_000_000_000;
    const FINAL_RAISE_AMOUNT: u64 = 1_000_000;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        performance_pool_base_tokens: PERFORMANCE_POOL,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;

    let ix_config = accounts.configure_performance_package_ix(0, 2, PACKAGE_ALLOCATION);
    harness.send(ix_config, &accounts.admin)?;

    let ix_config_over = accounts.configure_performance_package_ix(1, 4, PERFORMANCE_POOL);
    let result = harness.send(ix_config_over, &accounts.admin);
    assert!(result.is_err(), "over allocation must fail");

    harness.send(accounts.start_ix(), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.total_committed_amount = FINAL_RAISE_AMOUNT;
    })?;

    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        FINAL_RAISE_AMOUNT,
    )?;

    harness.send(accounts.complete_ix(FINAL_RAISE_AMOUNT), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.unix_timestamp_closed = Some(0);
    })?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    let initial_price = decoded.initial_token_price.unwrap();
    let target_price_2x = initial_price * 2;

    let ix_unlock = accounts.unlock_performance_package_ix(0, target_price_2x);
    harness.send(ix_unlock, &accounts.admin)?;

    let recipient_base = Pubkey::new_unique();
    harness.create_mock_token_account(recipient_base, accounts.base_mint, accounts.admin.pubkey())?;

    let ix_claim = accounts.claim_performance_package_ix(0, recipient_base);
    harness.send(ix_claim, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert!(decoded.performance_packages[0].is_claimed);

    Ok(())
}

#[test]
fn performance_package_multiplier_works() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    const PERFORMANCE_POOL: u64 = 2_000_000_000_000;
    const PACKAGE_ALLOCATION: u64 = 500_000_000_000;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        performance_pool_base_tokens: PERFORMANCE_POOL,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;

    harness.send(accounts.configure_performance_package_ix(0, 2, PACKAGE_ALLOCATION), &accounts.admin)?;
    harness.send(accounts.configure_performance_package_ix(1, 4, PACKAGE_ALLOCATION), &accounts.admin)?;
    harness.send(accounts.configure_performance_package_ix(2, 8, PACKAGE_ALLOCATION), &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.performance_packages[0].multiplier, 2);
    assert_eq!(decoded.performance_packages[1].multiplier, 4);
    assert_eq!(decoded.performance_packages[2].multiplier, 8);

    Ok(())
}

#[test]
fn performance_package_price_target_validation() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    const PERFORMANCE_POOL: u64 = 2_000_000_000_000;
    const PACKAGE_ALLOCATION: u64 = 500_000_000_000;
    const FINAL_RAISE_AMOUNT: u64 = 1_000_000;

    let config = InitConfig {
        minimum_raise_amount: 500_000,
        performance_pool_base_tokens: PERFORMANCE_POOL,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.configure_performance_package_ix(0, 2, PACKAGE_ALLOCATION), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    harness.update_kickstarter(&accounts.kickstarter_pda, |ks| {
        ks.total_committed_amount = FINAL_RAISE_AMOUNT;
    })?;
    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        FINAL_RAISE_AMOUNT,
    )?;

    harness.send(accounts.complete_ix(FINAL_RAISE_AMOUNT), &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    let initial_price = decoded.initial_token_price.unwrap();
    let below_target = initial_price + 1;

    let ix_unlock = accounts.unlock_performance_package_ix(0, below_target);
    let result = harness.send(ix_unlock, &accounts.admin);
    assert!(result.is_err(), "unlock should fail when price is below target");

    Ok(())
}

