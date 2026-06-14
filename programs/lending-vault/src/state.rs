use anchor_lang::prelude::*;

#[account]
pub struct LendingVault {
    pub authority:               Pubkey, // 32
    pub usdc_mint:               Pubkey, // 32
    pub total_liquidity:         u64,    // 8
    pub available_liquidity:     u64,    // 8
    pub total_loans_outstanding: u64,    // 8
    pub borrow_fee_bps:          u16,    // 2  (10 = 0.1%)
    pub bump:                    u8,     // 1
}

impl LendingVault {
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 2 + 1; // 99

    pub fn utilization_bps(&self) -> u64 {
        if self.total_liquidity == 0 { return 0; }
        self.total_loans_outstanding * 10_000 / self.total_liquidity
    }

    pub fn can_issue(&self, amount: u64) -> bool {
        self.utilization_bps() < 8_000 && self.available_liquidity >= amount
    }
}

#[account]
pub struct LoanAccount {
    pub borrower:         Pubkey,     // 32
    pub collateral_vault: Pubkey,     // 32
    pub principal_usd:    u64,        // 8
    pub fee_usd:          u64,        // 8
    pub opened_at:        i64,        // 8
    pub expires_at:       i64,        // 8
    pub status:           LoanStatus, // 1
    pub bump:             u8,         // 1
}

impl LoanAccount {
    pub const LEN: usize       = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1; // 106
    pub const DURATION_SECS: i64 = 86_400;       // 24 h
    pub const MIN_USDC: u64     = 10_000_000;    // $10 with 6 decimals

    pub fn total_owed(&self) -> u64 { self.principal_usd + self.fee_usd }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum LoanStatus { Active, Repaid, Liquidated }
