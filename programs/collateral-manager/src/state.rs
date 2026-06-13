use anchor_lang::prelude::*;

#[account]
pub struct CollateralVault {
    pub owner: Pubkey,              // 32
    pub lp_position_mint: Pubkey,  // 32
    pub lp_protocol: LpProtocol,   // 1
    pub collateral_usd_value: u64, // 8  (6-decimal, 1_000_000 = $1.00)
    pub max_borrow_usd: u64,       // 8
    pub active_loan: Option<Pubkey>, // 33
    pub deposited_at: i64,         // 8
    pub bump: u8,                  // 1
}

impl CollateralVault {
    pub const LEN: usize = 8 + 32 + 32 + 1 + 8 + 8 + 33 + 8 + 1; // 131

    pub fn max_borrow_from(usd_value: u64) -> u64 {
        usd_value / 2 // 50% LTV
    }

    /// Returns true when outstanding loan > 75% of collateral value.
    pub fn is_liquidatable(&self, outstanding_usd: u64) -> bool {
        outstanding_usd * 100 > self.collateral_usd_value * 75
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum LpProtocol {
    Meteora,
    Raydium,
    Orca,
}
