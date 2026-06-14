use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::delegate;
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use lending_vault::state::{LoanAccount, LoanStatus};
use crate::state::{TradeSession, TradeDirection, SessionStatus};
use crate::error::TradingError;

#[delegate]
#[derive(Accounts)]
pub struct OpenSession<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init, del,
        payer  = payer,
        space  = TradeSession::LEN,
        seeds  = [b"trade_session", loan_account.key().as_ref()],
        bump,
    )]
    pub trade_session: Account<'info, TradeSession>,

    /// LoanAccount from lending-vault — must be Active
    #[account(
        constraint = loan_account.borrower == payer.key(),
        constraint = loan_account.status == LoanStatus::Active @ TradingError::LoanNotActive,
        owner = lending_vault::ID,
    )]
    pub loan_account: Account<'info, LoanAccount>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<OpenSession>,
    market: Pubkey,
    direction: TradeDirection,
    size_usd: u64,
    leverage: u8,
    flash_trade_position_id: [u8; 32],
    entry_price: u64,
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
    ts.expires_at              = clock.unix_timestamp + TradeSession::DURATION_SECS;
    ts.status                  = SessionStatus::Open;
    ts.bump                    = ctx.bumps.trade_session;

    // Delegate trade_session PDA to the ER validator.
    // remaining_accounts[0] is the ER validator pubkey.
    ctx.accounts.delegate_trade_session(
        &ctx.accounts.payer,
        &[b"trade_session", ctx.accounts.loan_account.key().as_ref()],
        DelegateConfig {
            validator: ctx.remaining_accounts.first().map(|a| a.key()),
            ..Default::default()
        },
    )?;

    Ok(())
}
