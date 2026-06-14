use anchor_lang::prelude::*;

#[account]
pub struct TradeSession {
    pub loan_account:            Pubkey,          // 32
    pub flash_trade_position_id: [u8; 32],        // 32
    pub market:                  Pubkey,           // 32
    pub direction:               TradeDirection,   // 1
    pub size_usd:                u64,              // 8
    pub leverage:                u8,               // 1
    pub entry_price:             u64,              // 8  (6-decimal USD)
    pub current_price:           u64,              // 8
    pub take_profit:             Option<u64>,      // 9
    pub stop_loss:               Option<u64>,      // 9
    pub trigger_order_id:        Option<[u8; 32]>, // 33
    pub opened_at:               i64,              // 8
    pub expires_at:              i64,              // 8
    pub status:                  SessionStatus,    // 1
    pub bump:                    u8,               // 1
}

impl TradeSession {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 1 + 8 + 1 + 8 + 8 + 9 + 9 + 33 + 8 + 8 + 1 + 1; // 199

    pub const DURATION_SECS: i64 = 86_400;

    pub fn unrealized_pnl(&self) -> i64 {
        let entry   = self.entry_price as i64;
        let current = self.current_price as i64;
        let size    = self.size_usd as i64;
        match self.direction {
            TradeDirection::Long  => (current - entry) * size / entry,
            TradeDirection::Short => (entry - current) * size / entry,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum TradeDirection { Long, Short }

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum SessionStatus { Open, Settling, Settled }
