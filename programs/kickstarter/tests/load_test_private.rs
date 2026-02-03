mod support;

use solana_keypair::Keypair;
use support::{
    fund_private_ix, private_claim_ix, start_private_round_ix,
    finalize_private_round_ix, InitConfig, KickstarterAccounts,
    Pubkey, Signer, TestHarness, TestResult,
};

#[test]
fn load_test_50_private_investors() -> TestResult {
    let mut harness = TestHarness::new()?;
    let accounts = KickstarterAccounts::generate(&mut harness)?;

    // Создаем 50 инвесторов
    let investors: Vec<Keypair> = (0..50).map(|_| Keypair::new()).collect();
    for investor in &investors {
        harness.airdrop(&investor.pubkey(), 10_000_000_000)?;
    }

    let config = InitConfig::default();
    harness.send(accounts.initialize_ix(config), &accounts.admin)?;
    harness.send(accounts.start_ix(), &accounts.admin)?;
    harness.send(start_private_round_ix(&accounts.admin.pubkey(), &accounts.kickstarter_pda), &accounts.admin)?;

    println!("🚀 Starting load test with 50 private investors...");

    let start_time = std::time::Instant::now();
    let mut total_committed = 0u64;

    // Пакетное приватное финансирование
    for (i, investor) in investors.iter().enumerate() {
        let amount = 100_000 + (i as u64 * 10_000); // Разные суммы
        let salt = [i as u8; 32]; // Уникальный salt для каждого

        let ix = fund_private_ix(
            &investor.pubkey(),
            &accounts.kickstarter_pda,
            &accounts.private_state,
            amount,
            salt,
        );
        harness.send(ix, investor)?;

        total_committed += amount;

        if (i + 1) % 10 == 0 {
            println!("✅ Processed {} investors, total committed: {} USDC", i + 1, total_committed / 1_000_000);
        }
    }

    let funding_duration = start_time.elapsed();
    println!("⏱️  50 private funding transactions took: {:?}", funding_duration);

    // Финализируем приватный раунд
    let final_private_state = harness.private_state(&accounts.private_state)?;
    harness.send(finalize_private_round_ix(
        &accounts.admin.pubkey(),
        &accounts.kickstarter_pda,
        &accounts.private_state,
        final_private_state.commitments_root,
        final_private_state.committed_amount,
        [0u8; 64],
    ), &accounts.admin)?;

    harness.send(accounts.complete_ix(total_committed), &accounts.admin)?;

    // Пакетное приватное claim'инг
    let claim_start_time = std::time::Instant::now();

    for (i, investor) in investors.iter().enumerate() {
        let amount = 100_000 + (i as u64 * 10_000);
        let salt = [i as u8; 32];

        let user_base = Pubkey::new_unique();
        harness.create_mock_token_account(user_base, accounts.base_mint, investor.pubkey())?;

        let ix = private_claim_ix(
            &investor.pubkey(),
            &accounts.kickstarter_pda,
            &accounts.private_state,
            &accounts.base_vault,
            &user_base,
            amount,
            salt,
        );
        harness.send(ix, investor)?;
    }

    let claim_duration = claim_start_time.elapsed();
    println!("⏱️  50 private claim transactions took: {:?}", claim_duration);

    // Метрики производительности
    let total_duration = start_time.elapsed();
    let avg_funding_latency = funding_duration.as_millis() as f64 / 50.0;
    let avg_claim_latency = claim_duration.as_millis() as f64 / 50.0;

    println!("📊 Performance Metrics:");
    println!("   Total duration: {:?}", total_duration);
    println!("   Avg funding latency: {:.2}ms per txn", avg_funding_latency);
    println!("   Avg claim latency: {:.2}ms per txn", avg_claim_latency);
    println!("   Total committed: {} USDC", total_committed / 1_000_000);
    println!("   Private investors: {}", final_private_state.investor_count);

    // Проверки
    assert_eq!(final_private_state.investor_count, 50);
    assert_eq!(final_private_state.committed_amount, total_committed);
    assert!(avg_funding_latency < 1000.0); // < 1 секунда в среднем
    assert!(avg_claim_latency < 1000.0);   // < 1 секунда в среднем

    println!("✅ Load test completed successfully!");

    Ok(())
}