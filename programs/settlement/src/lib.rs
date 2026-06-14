use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("5Ymdq7VQKyhnerpYBaoc3pgV3CYdXHhGbBWQTVwf463k");

#[program]
pub mod settlement {
    use super::*;

    /// Settle a closed trade: repay lending-vault loan and record the result.
    pub fn settle(ctx: Context<Settle>, realized_pnl: i64) -> Result<()> {
        instructions::settle::handler(ctx, realized_pnl)
    }
}
