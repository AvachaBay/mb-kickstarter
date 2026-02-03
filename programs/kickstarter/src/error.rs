use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Kickstarter is not initialized")]
    InvalidKickstarterState,
    #[msg("Invalid admin")]
    InvalidAdmin,
    #[msg("Time of Funding phase hasn't finished yet")]
    TooEarlyToCompleteKickstarter,
    #[msg("Can't fund. The time for funding is already out.")]
    FundingTimeIsOut,
    #[msg("The funding exceeds the hard cap")]
    OverHardcapLimit,
    #[msg("Performance package index is invalid")]
    InvalidPerformancePackageIndex,
    #[msg("Performance package allocation is invalid")]
    InvalidPerformancePackageAllocation,
    #[msg("Performance package already configured")]
    PerformancePackageAlreadyConfigured,
    #[msg("Performance package is not configured")]
    PerformancePackageNotConfigured,
    #[msg("Performance package already unlocked")]
    PerformancePackageAlreadyUnlocked,
    #[msg("Performance package is locked")]
    PerformancePackageLocked,
    #[msg("Performance package already claimed")]
    PerformancePackageAlreadyClaimed,
    #[msg("Performance pool allocation exceeded")]
    PerformancePoolExceeded,
    #[msg("Invalid final raise amount")]
    InvalidFinalRaiseAmount,
    #[msg("Final raise amount exceeds total committed funds")]
    FinalAmountExceedsTotalCommitted,
    #[msg("Final raise amount is missing")]
    FinalRaiseAmountMissing,
    #[msg("Committed amount snapshot is missing")]
    CommittedSnapshotMissing,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Treasury token account owner mismatch")]
    InvalidTreasuryAccountOwner,
    #[msg("Token account mint mismatch")]
    InvalidQuoteMint,
    #[msg("Base token account mint mismatch")]
    InvalidBaseMint,
    #[msg("Invalid minimum raise amount")]
    InvalidMinimumRaiseAmount,
    #[msg("Price target not reached for this performance package")]
    PriceTargetNotReached,
    #[msg("Initial token price not set")]
    InitialTokenPriceNotSet,
    #[msg("Previous performance package must be unlocked first")]
    PreviousPackageNotUnlocked,
    #[msg("Too early to unlock this performance package")]
    TooEarlyToUnlockPackage,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Invalid commitments root")]
    InvalidCommitmentsRoot,
    #[msg("Invalid attested amount")]
    InvalidAttestedAmount,
    #[msg("Double spend detected")]
    DoubleSpend,
}
