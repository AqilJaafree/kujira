use anchor_lang::prelude::*;
use crate::state::{TradeSession, SessionStatus};
use crate::error::TradingError;

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    pub keeper: Signer<'info>,

    #[account(
        mut,
        seeds = [b"trade_session", trade_session.loan_account.as_ref()],
        bump  = trade_session.bump,
        constraint = trade_session.status == SessionStatus::Open @ TradingError::SessionNotOpen,
    )]
    pub trade_session: Account<'info, TradeSession>,
}

pub fn handler(ctx: Context<UpdatePrice>, price: u64) -> Result<()> {
    ctx.accounts.trade_session.current_price = price;
    Ok(())
}
