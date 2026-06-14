use anchor_lang::prelude::*;
use crate::state::{TradeSession, SessionStatus};
use crate::error::TradingError;

/// Called by keeper on base layer when session has expired (24 h).
/// Does NOT use ER — trade_session must already be undelegated.
#[derive(Accounts)]
pub struct ForceCloseSession<'info> {
    pub keeper: Signer<'info>,

    #[account(
        mut,
        seeds  = [b"trade_session", trade_session.loan_account.as_ref()],
        bump   = trade_session.bump,
        constraint = trade_session.status == SessionStatus::Open @ TradingError::SessionNotOpen,
    )]
    pub trade_session: Account<'info, TradeSession>,
}

pub fn handler(ctx: Context<ForceCloseSession>, _realized_pnl: i64) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.trade_session.expires_at,
        TradingError::SessionNotExpired,
    );
    ctx.accounts.trade_session.status = SessionStatus::Settling;
    Ok(())
}
