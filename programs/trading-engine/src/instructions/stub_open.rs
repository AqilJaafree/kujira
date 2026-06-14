/// Localhost-only: creates a TradeSession without the ER delegation CPI.
/// Use in tests to seed session state for update_price / force_close / settle.
use anchor_lang::prelude::*;
use lending_vault::state::{LoanAccount, LoanStatus};
use crate::state::{TradeSession, TradeDirection, SessionStatus};
use crate::error::TradingError;

#[derive(Accounts)]
pub struct StubOpenSession<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = TradeSession::LEN,
        seeds = [b"trade_session", loan_account.key().as_ref()],
        bump,
    )]
    pub trade_session: Account<'info, TradeSession>,

    #[account(
        constraint = loan_account.borrower == payer.key(),
        constraint = loan_account.status == LoanStatus::Active @ TradingError::LoanNotActive,
        owner = lending_vault::ID,
    )]
    pub loan_account: Account<'info, LoanAccount>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<StubOpenSession>,
    market: Pubkey,
    direction: TradeDirection,
    size_usd: u64,
    leverage: u8,
    flash_trade_position_id: [u8; 32],
    entry_price: u64,
    // Optionally pre-expire the session for force_close tests (0 = normal expiry)
    override_expires_at: i64,
) -> Result<()> {
    require!(leverage >= 1 && leverage <= 10, TradingError::InvalidLeverage);
    require!(size_usd <= ctx.accounts.loan_account.principal_usd, TradingError::SizeExceedsLoan);

    let clock = Clock::get()?;
    let ts    = &mut ctx.accounts.trade_session;
    ts.loan_account            = ctx.accounts.loan_account.key();
    ts.flash_trade_position_id = flash_trade_position_id;
    ts.market                  = market;
    ts.direction               = direction;
    ts.size_usd                = size_usd;
    ts.leverage                = leverage;
    ts.entry_price             = entry_price;
    ts.current_price           = entry_price;
    ts.take_profit             = None;
    ts.stop_loss               = None;
    ts.trigger_order_id        = None;
    ts.opened_at               = clock.unix_timestamp;
    ts.expires_at              = if override_expires_at == 0 {
        clock.unix_timestamp + TradeSession::DURATION_SECS
    } else {
        override_expires_at
    };
    ts.status = SessionStatus::Open;
    ts.bump   = ctx.bumps.trade_session;

    Ok(())
}
