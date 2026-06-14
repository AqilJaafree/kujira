use anchor_lang::prelude::*;
use crate::state::{TradeSession, SessionStatus};
use crate::error::TradingError;

#[derive(Accounts)]
pub struct MarkSettled<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        constraint = trade_session.status == SessionStatus::Settling
            @ TradingError::SessionNotSettling,
    )]
    pub trade_session: Account<'info, TradeSession>,
}

pub fn handler(ctx: Context<MarkSettled>) -> Result<()> {
    ctx.accounts.trade_session.status = SessionStatus::Settled;
    Ok(())
}
