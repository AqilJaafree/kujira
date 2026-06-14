use anchor_lang::prelude::*;

#[account]
pub struct SettlementRecord {
    pub trade_session: Pubkey,   // 32
    pub loan_account:  Pubkey,   // 32
    pub realized_pnl:  i64,      // 8
    pub repaid_amount: u64,      // 8
    pub settled_at:    i64,      // 8
    pub bump:          u8,       // 1
}

impl SettlementRecord {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 1; // 97
}
