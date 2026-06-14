use anchor_lang::prelude::*;

#[error_code]
pub enum SettlementError {
    #[msg("Trade session is not in Settling state")]
    SessionNotSettling,
    #[msg("Repayment amount is zero")]
    ZeroRepayment,
    #[msg("Settlement already exists for this session")]
    AlreadySettled,
}
