use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Loan amount is below the $10 minimum")]
    BelowMinimum,
    #[msg("Loan amount exceeds 50% LTV of collateral")]
    ExceedsMaxBorrow,
    #[msg("Vault utilisation is above 80%; no new loans")]
    VaultAtCapacity,
    #[msg("Collateral vault already has an active loan")]
    CollateralAlreadyLoaned,
    #[msg("Loan is not in Active status")]
    LoanNotActive,
    #[msg("Loan has not expired yet")]
    LoanNotExpired,
}
