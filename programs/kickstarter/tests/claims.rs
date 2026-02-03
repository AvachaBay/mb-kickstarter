mod support;

use anchor_lang::solana_program::program_pack::Pack;
use kickstarter::state::KickstarterState;
use solana_keypair::Keypair;
use support::{
    claim_ix, fund_ix, refund_ix, InitConfig, KickstarterAccounts, Pubkey,
    Signer, TestHarness, TestResult,
};

#[test]
fn claim_proportional_distribution_single_user() -> TestResult {
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

    let user_quote = Pubkey::new_unique();
    harness.set_token_account_balance(user_quote, accounts.quote_mint, user.pubkey(), 5_000_000)?;

    let ix_fund = fund_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &user_quote,
        &accounts.quote_vault,
        5_000_000,
    );
    harness.send(ix_fund, &user)?;

    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        5_000_000,
    )?;

    harness.send(accounts.complete_ix(5_000_000), &accounts.admin)?;

    let ks_state = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    let expected_claim = ks_state.total_base_tokens_for_investors;

    let user_base = Pubkey::new_unique();
    harness.create_mock_token_account(user_base, accounts.base_mint, user.pubkey())?;

    let ix_claim = claim_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.base_vault,
        &user_base,
    );
    harness.send(ix_claim, &user)?;

    let user_base_acc = harness.account(&user_base).unwrap();
    let claimed =
        anchor_spl::token::spl_token::state::Account::unpack(&user_base_acc.data)?.amount;
    assert_eq!(claimed, expected_claim);

    Ok(())
}

#[test]
fn refund_returns_funds() -> TestResult {
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

    let user_quote = Pubkey::new_unique();
    harness.set_token_account_balance(user_quote, accounts.quote_mint, user.pubkey(), 500_000)?;

    let ix_fund = fund_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &user_quote,
        &accounts.quote_vault,
        500_000,
    );
    harness.send(ix_fund, &user)?;

    harness.set_token_account_balance(
        accounts.quote_vault,
        accounts.quote_mint,
        accounts.kickstarter_pda,
        500_000,
    )?;

    harness.send(accounts.complete_ix(0), &accounts.admin)?;

    let ks_state = harness.kickstarter_state(&accounts.kickstarter_pda)?;
    assert_eq!(ks_state.state, KickstarterState::Refunding);

    let ix_refund = refund_ix(
        &user.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.quote_vault,
        &user_quote,
    );
    harness.send(ix_refund, &user)?;

    let user_quote_acc = harness.account(&user_quote).unwrap();
    let refunded =
        anchor_spl::token::spl_token::state::Account::unpack(&user_quote_acc.data)?.amount;
    assert_eq!(refunded, 500_000);

    Ok(())
}

#[test]
fn close_kickstarter_from_initialized() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    harness.send(accounts.initialize_ix(InitConfig::default()), &accounts.admin)?;
    harness.send(accounts.close_ix(), &accounts.admin)?;

    assert!(harness.account(&accounts.kickstarter_pda).is_none());

    Ok(())
}

#[test]
fn double_start_fails() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    harness.send(accounts.initialize_ix(InitConfig::default()), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;

    let result = harness.send(accounts.start_ix(), &accounts.admin);
    assert!(result.is_err());

    Ok(())
}

