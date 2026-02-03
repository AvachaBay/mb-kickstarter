mod support;

use anchor_lang::solana_program::program_pack::Pack;
use kickstarter::state::KickstarterState;
use solana_keypair::Keypair;
use support::{InitConfig, KickstarterAccounts, Pubkey, Signer, TestHarness, TestResult};

#[test]
fn set_minimum_raise_succeeds() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let config = InitConfig {
        minimum_raise_amount: 1_000_000,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;

    let new_minimum = 2_000_000;
    let ix = accounts.set_minimum_raise_ix(new_minimum);
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.minimum_raise_amount, new_minimum);
    assert_eq!(decoded.hard_cap, u64::MAX);

    Ok(())
}

#[test]
fn set_minimum_raise_from_live_state_succeeds() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    let config = InitConfig {
        minimum_raise_amount: 1_000_000,
        ..Default::default()
    };
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    let new_minimum = 3_000_000;
    let ix = accounts.set_minimum_raise_ix(new_minimum);
    harness.send(ix, &accounts.admin)?;

    let decoded = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(decoded.minimum_raise_amount, new_minimum);
    assert_eq!(decoded.state, KickstarterState::Live);

    Ok(())
}

#[test]
fn set_minimum_raise_zero_fails() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    harness.send(accounts.initialize_ix(InitConfig::default()), &accounts.admin)?;

    let ix = accounts.set_minimum_raise_ix(0);
    let result = harness.send(ix, &accounts.admin);
    assert!(result.is_err(), "Setting minimum raise to 0 should fail");

    Ok(())
}

#[test]
fn set_minimum_raise_from_complete_state_fails() -> TestResult {
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

    harness.send(accounts.complete_ix(FINAL_RAISE_AMOUNT), &accounts.admin)?;

    let ix = accounts.set_minimum_raise_ix(2_000_000);
    let result = harness.send(ix, &accounts.admin);
    assert!(
        result.is_err(),
        "Setting minimum raise from Complete state should fail"
    );

    Ok(())
}

#[test]
fn stake_from_treasury_succeeds() -> TestResult {
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

    harness.send(accounts.complete_ix(FINAL_RAISE_AMOUNT), &accounts.admin)?;

    // Set treasury balance for staking
    // Note: treasury_token_account owner should be admin (treasury = admin in CLI)
    const TREASURY_BALANCE: u64 = 500_000;
    harness.set_token_account_balance(
        accounts.treasury_token_account,
        accounts.quote_mint,
        accounts.admin.pubkey(),
        TREASURY_BALANCE,
    )?;

    let staking_account = Pubkey::new_unique();
    harness.create_mock_token_account(
        staking_account,
        accounts.quote_mint,
        accounts.admin.pubkey(),
    )?;

    const STAKE_AMOUNT: u64 = 200_000;
    let ix = accounts.stake_from_treasury_ix(staking_account, STAKE_AMOUNT);
    harness.send(ix, &accounts.admin)?;

    let staking_acc = harness.account(&staking_account).unwrap();
    let staked_amount =
        anchor_spl::token::spl_token::state::Account::unpack(&staking_acc.data)?.amount;
    assert_eq!(staked_amount, STAKE_AMOUNT);

    Ok(())
}

#[test]
fn stake_from_treasury_insufficient_balance_fails() -> TestResult {
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

    harness.send(accounts.complete_ix(FINAL_RAISE_AMOUNT), &accounts.admin)?;

    // трежери меньше чем стейк аккаунт, надо чтобы выполняющий акк был админом
    const TREASURY_BALANCE: u64 = 100_000;
    harness.set_token_account_balance(
        accounts.treasury_token_account,
        accounts.quote_mint,
        accounts.admin.pubkey(),
        TREASURY_BALANCE,
    )?;

    let staking_account = Pubkey::new_unique();
    harness.create_mock_token_account(
        staking_account,
        accounts.quote_mint,
        accounts.admin.pubkey(),
    )?;

    const STAKE_AMOUNT: u64 = 200_000; // More than treasury balance
    let ix = accounts.stake_from_treasury_ix(staking_account, STAKE_AMOUNT);
    let result = harness.send(ix, &accounts.admin);
    assert!(
        result.is_err(),
        "Staking more than treasury balance should fail"
    );

    Ok(())
}

#[test]
fn stake_from_treasury_zero_amount_fails() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    harness.send(accounts.initialize_ix(InitConfig::default()), &accounts.admin)?;

    let staking_account = Pubkey::new_unique();
    harness.create_mock_token_account(
        staking_account,
        accounts.quote_mint,
        accounts.admin.pubkey(),
    )?;

    let ix = accounts.stake_from_treasury_ix(staking_account, 0);
    let result = harness.send(ix, &accounts.admin);
    assert!(result.is_err(), "Staking zero amount should fail");

    Ok(())
}

