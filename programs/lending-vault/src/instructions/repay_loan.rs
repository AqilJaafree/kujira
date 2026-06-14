use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{LendingVault, LoanAccount, LoanStatus};
use crate::error::LendingError;

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    /// Caller may be settlement program (CPI) or borrower directly
    pub caller: Signer<'info>,

    #[account(mut, seeds = [b"lending_vault"], bump = lending_vault.bump)]
    pub lending_vault: Account<'info, LendingVault>,

    #[account(
        mut,
        seeds = [b"loan", loan_account.collateral_vault.as_ref()],
        bump  = loan_account.bump,
        constraint = loan_account.status == LoanStatus::Active @ LendingError::LoanNotActive,
    )]
    pub loan_account: Account<'info, LoanAccount>,

    #[account(
        mut,
        seeds = [b"vault_usdc", lending_vault.key().as_ref()],
        bump,
        token::mint      = lending_vault.usdc_mint,
        token::authority = lending_vault,
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    /// Source of repayment (settlement escrow or borrower's own ATA)
    #[account(
        mut,
        constraint = repayment_source.mint == lending_vault.usdc_mint,
    )]
    pub repayment_source: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<RepayLoan>, proceeds: u64) -> Result<()> {
    let owed  = ctx.accounts.loan_account.total_owed();
    let repay = proceeds.min(owed);

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.repayment_source.to_account_info(),
                to:        ctx.accounts.vault_usdc_ata.to_account_info(),
                authority: ctx.accounts.caller.to_account_info(),
            },
        ),
        repay,
    )?;

    let principal = ctx.accounts.loan_account.principal_usd;
    let lv = &mut ctx.accounts.lending_vault;
    lv.available_liquidity     += principal;
    lv.total_loans_outstanding -= principal;

    ctx.accounts.loan_account.status = LoanStatus::Repaid;
    Ok(())
}

// -----------------------------------------------------------------------
// Force expire: callable by keeper after expires_at
// -----------------------------------------------------------------------
#[derive(Accounts)]
pub struct ForceExpireLoan<'info> {
    pub keeper: Signer<'info>,

    #[account(mut, seeds = [b"lending_vault"], bump = lending_vault.bump)]
    pub lending_vault: Account<'info, LendingVault>,

    #[account(
        mut,
        seeds = [b"loan", loan_account.collateral_vault.as_ref()],
        bump  = loan_account.bump,
        constraint = loan_account.status == LoanStatus::Active @ LendingError::LoanNotActive,
    )]
    pub loan_account: Account<'info, LoanAccount>,
}

pub fn force_expire_handler(ctx: Context<ForceExpireLoan>) -> Result<()> {
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= ctx.accounts.loan_account.expires_at,
        LendingError::LoanNotExpired
    );
    let principal = ctx.accounts.loan_account.principal_usd;
    let lv = &mut ctx.accounts.lending_vault;
    lv.total_loans_outstanding -= principal;
    ctx.accounts.loan_account.status = LoanStatus::Liquidated;
    Ok(())
}
