use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::ephem::{MagicIntentBundleBuilder, FoldableIntentBuilder};
use crate::state::{TradeSession, SessionStatus};
use crate::error::TradingError;

#[derive(Accounts)]
pub struct CloseSession<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds  = [b"trade_session", trade_session.loan_account.as_ref()],
        bump   = trade_session.bump,
        constraint = trade_session.status == SessionStatus::Open @ TradingError::SessionNotOpen,
    )]
    pub trade_session: Account<'info, TradeSession>,

    /// CHECK: MagicBlock magic_context — required by ER SDK
    #[account(mut)]
    pub magic_context: UncheckedAccount<'info>,

    /// CHECK: MagicBlock magic_program
    pub magic_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<CloseSession>, _realized_pnl: i64) -> Result<()> {
    ctx.accounts.trade_session.status = SessionStatus::Settling;

    // Commit account state to base layer and end the ER session.
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[ctx.accounts.trade_session.to_account_info()])
    .build_and_invoke()?;

    Ok(())
}
