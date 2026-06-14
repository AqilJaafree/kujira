use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("FVpeDwQgf7GSz8CQachQXzZZJrKE9sJ2FWbuoHLuQkDs");

// #[ephemeral] injects the undelegation callback required by the ER SDK
#[ephemeral]
#[program]
pub mod trading_engine {
    use super::*;

    /// Base layer: initialise TradeSession + delegate to ER
    pub fn open_session(
        ctx: Context<OpenSession>,
        market: Pubkey,
        direction: state::TradeDirection,
        size_usd: u64,
        leverage: u8,
        flash_trade_position_id: [u8; 32],
        entry_price: u64,
    ) -> Result<()> {
        instructions::open_session::handler(
            ctx, market, direction, size_usd, leverage,
            flash_trade_position_id, entry_price,
        )
    }

    /// ER only: keeper crank updates current price each slot
    pub fn update_price(ctx: Context<UpdatePrice>, price: u64) -> Result<()> {
        instructions::update_price::handler(ctx, price)
    }

    /// ER only: close position + commit_and_undelegate
    pub fn close_session(ctx: Context<CloseSession>, realized_pnl: i64) -> Result<()> {
        instructions::close_session::handler(ctx, realized_pnl)
    }

    /// Base layer: keeper force-closes expired session
    pub fn force_close_session(ctx: Context<ForceCloseSession>, realized_pnl: i64) -> Result<()> {
        instructions::force_close::handler(ctx, realized_pnl)
    }
}
