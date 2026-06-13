use anchor_lang::prelude::*;

#[error_code]
pub enum CollateralError {
    #[msg("Cannot withdraw while an active loan exists")]
    ActiveLoanExists,
    #[msg("LP token account has zero balance")]
    InsufficientLpBalance,
    #[msg("Collateral LTV is healthy; liquidation not permitted")]
    LtvHealthy,
    #[msg("Collateral vault already has an active loan")]
    AlreadyLoaned,
}
