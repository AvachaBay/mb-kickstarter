use anchor_lang::prelude::*;
use std::fmt::Debug;

use crate::constants::MAX_PERFORMANCE_PACKAGES;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum KickstarterState {
    Initialized,
    Live,
    Closed,
    Complete,
    Refunding,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Default)]
pub struct PerformancePackage {
    pub multiplier: u8,
    pub allocation: u64,
    pub is_configured: bool,
    pub is_unlocked: bool,
    pub is_claimed: bool,
    pub unlocked_at: Option<i64>,
}

#[account]
#[derive(InitSpace)]
pub struct Kickstarter {
    pub pda_bump: u8,
    pub state: KickstarterState,
    pub kickstarter_authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub treasury: Pubkey,
    pub minimum_raise_amount: u64,
    pub total_base_tokens_for_investors: u64,
    pub total_committed_amount: u64,
    /// независимое от min_raise_amount, служит "потолком-лимитом"" для пополнеия от fund'еров
    pub hard_cap: u64, //hard cap in quote tokens
    pub final_raise_amount: Option<u64>,
    pub total_committed_at_completion: Option<u64>,
    pub seconds_for_launch: u32,
    pub unix_timestamp_started: Option<i64>,
    pub unix_timestamp_closed: Option<i64>,
    pub performance_pool_base_tokens: u64,
    pub configured_performance_tokens: u64,
    pub performance_packages: [PerformancePackage; MAX_PERFORMANCE_PACKAGES],
    pub monthly_team_spending_usdc: u64,
    pub package_unlock_delay_seconds: i64,
    pub calculated_liquidity_amount: Option<u64>,
    pub initial_token_price: Option<u64>,
    pub calculated_base_tokens_for_investors: Option<u64>,
    pub calculated_base_tokens_for_liquidity: Option<u64>,
    pub calculated_performance_pool_tokens: Option<u64>,
    pub private_commitments_root: [u8; 32],
    pub private_investor_count: u32,
    pub is_private_round_active: bool,
}

impl Debug for KickstarterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialized => write!(f, "Initialized"),
            Self::Live => write!(f, "Live"),
            Self::Closed => write!(f, "Closed"),
            Self::Complete => write!(f, "Complete"),
            Self::Refunding => write!(f, "Refunding"),
        }
    }
}


