use anchor_lang::prelude::*;

#[constant]
pub const SEED_KICKSTARTER: &str = "kickstarter";

#[constant]
pub const SEED_FUNDER_POSITION: &str = "funder_position";

#[constant]
pub const SEED_BASE_VAULT: &str = "base_vault";

#[constant]
pub const SEED_QUOTE_VAULT: &str = "quote_vault";

#[constant]
pub const SEED_PRIVATE_STATE: &str = "private_state";

#[constant]
pub const BPS_DENOMINATOR: u64 = 10_000;

#[constant]
pub const LIQUIDITY_BPS: u64 = 2_000; // 20%

pub const MAX_PERFORMANCE_PACKAGES: usize = 5;

#[constant]
pub const BASE_TOKENS_FOR_INVESTORS_BPS: u64 = 10_000; // 100% - базовое значение для расчета

#[constant]
pub const BASE_TOKENS_FOR_LIQUIDITY_BPS: u64 = 2_900; // 29% от базового supply для ликвидности

#[constant]
pub const BASE_TOKENS_FOR_PERFORMANCE_BPS: u64 = 10_000; // 100% от базового supply для performance pool (минимум)

pub const DEFAULT_PACKAGE_UNLOCK_DELAY_SECONDS: i64 = 60;    
