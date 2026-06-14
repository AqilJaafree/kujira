use anchor_lang::prelude::*;

#[error_code]
pub enum TradingError {
    #[msg("Loan is not active")]
    LoanNotActive,
    #[msg("Trade size exceeds loan principal")]
    SizeExceedsLoan,
    #[msg("Session is not open")]
    SessionNotOpen,
    #[msg("Session has not expired yet")]
    SessionNotExpired,
    #[msg("Invalid keeper signature")]
    InvalidKeeperSignature,
    #[msg("Leverage must be between 1 and 10")]
    InvalidLeverage,
}
