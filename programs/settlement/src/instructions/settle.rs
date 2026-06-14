use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use lending_vault::{
    cpi::{self as lv_cpi, accounts::RepayLoan},
    state::{LendingVault, LoanAccount, LoanStatus},
};
use trading_engine::state::{SessionStatus, TradeSession};
use crate::error::SettlementError;
use crate::state::SettlementRecord;

#[derive(Accounts)]
pub struct Settle<'info> {
    #[account(mut)]
    pub settler: Signer<'info>,

    #[account(
        init,
        payer  = settler,
        space  = SettlementRecord::LEN,
        seeds  = [b"settlement", trade_session.key().as_ref()],
        bump,
    )]
    pub settlement_record: Account<'info, SettlementRecord>,

    /// TradeSession owned by trading-engine — must be Settling
    #[account(
        mut,
        constraint = trade_session.status == SessionStatus::Settling
            @ SettlementError::SessionNotSettling,
        owner = trading_engine::ID,
    )]
    pub trade_session: Account<'info, TradeSession>,

    /// LoanAccount owned by lending-vault — must be Active
    #[account(
        mut,
        constraint = loan_account.status == LoanStatus::Active,
        owner = lending_vault::ID,
    )]
    pub loan_account: Account<'info, LoanAccount>,

    #[account(mut, owner = lending_vault::ID)]
    pub lending_vault: Account<'info, LendingVault>,

    /// Vault USDC ATA managed by lending-vault
    #[account(mut)]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    /// Settler's USDC ATA (proceeds from the closed flash.trade position)
    #[account(mut, constraint = repayment_source.owner == settler.key())]
    pub repayment_source: Account<'info, TokenAccount>,

    pub lending_vault_program: Program<'info, lending_vault::program::LendingVault>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Settle>, realized_pnl: i64) -> Result<()> {
    let clock = Clock::get()?;

    let repay_amount = ctx.accounts.loan_account.principal_usd
        .checked_add(ctx.accounts.loan_account.fee_usd)
        .unwrap();

    require!(repay_amount > 0, SettlementError::ZeroRepayment);

    // CPI to lending-vault::repay_loan — it transfers from repayment_source and marks Repaid
    lv_cpi::repay_loan(
        CpiContext::new(
            ctx.accounts.lending_vault_program.to_account_info(),
            RepayLoan {
                caller:           ctx.accounts.settler.to_account_info(),
                lending_vault:    ctx.accounts.lending_vault.to_account_info(),
                loan_account:     ctx.accounts.loan_account.to_account_info(),
                vault_usdc_ata:   ctx.accounts.vault_usdc_ata.to_account_info(),
                repayment_source: ctx.accounts.repayment_source.to_account_info(),
                token_program:    ctx.accounts.token_program.to_account_info(),
            },
        ),
        repay_amount,
    )?;

    ctx.accounts.trade_session.status = SessionStatus::Settled;

    let sr = &mut ctx.accounts.settlement_record;
    sr.trade_session  = ctx.accounts.trade_session.key();
    sr.loan_account   = ctx.accounts.loan_account.key();
    sr.realized_pnl   = realized_pnl;
    sr.repaid_amount  = repay_amount;
    sr.settled_at     = clock.unix_timestamp;
    sr.bump           = ctx.bumps.settlement_record;

    Ok(())
}
