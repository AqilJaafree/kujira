use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use collateral_manager::state::CollateralVault;
use crate::state::{LendingVault, LoanAccount, LoanStatus};
use crate::error::LendingError;

#[derive(Accounts)]
pub struct IssueLoan<'info> {
    #[account(mut)]
    pub borrower: Signer<'info>,

    #[account(mut, seeds = [b"lending_vault"], bump = lending_vault.bump)]
    pub lending_vault: Account<'info, LendingVault>,

    /// CollateralVault owned by collateral-manager program
    #[account(
        constraint = collateral_vault.owner == borrower.key(),
        constraint = collateral_vault.active_loan.is_none() @ LendingError::CollateralAlreadyLoaned,
        owner = collateral_manager::ID,
    )]
    pub collateral_vault: Account<'info, CollateralVault>,

    #[account(
        init,
        payer  = borrower,
        space  = LoanAccount::LEN,
        seeds  = [b"loan", collateral_vault.key().as_ref()],
        bump,
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

    #[account(
        mut,
        constraint = borrower_usdc_ata.owner == borrower.key(),
        constraint = borrower_usdc_ata.mint  == lending_vault.usdc_mint,
    )]
    pub borrower_usdc_ata: Account<'info, TokenAccount>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<IssueLoan>, amount: u64) -> Result<()> {
    let cv = &ctx.accounts.collateral_vault;
    require!(amount >= LoanAccount::MIN_USDC,             LendingError::BelowMinimum);
    require!(amount <= cv.max_borrow_usd,                 LendingError::ExceedsMaxBorrow);
    require!(ctx.accounts.lending_vault.can_issue(amount), LendingError::VaultAtCapacity);

    let clock = Clock::get()?;
    let fee   = amount * ctx.accounts.lending_vault.borrow_fee_bps as u64 / 10_000;

    // Snapshot vault bump before mutable borrow.
    let vault_bump = ctx.accounts.lending_vault.bump;
    let vault_info = ctx.accounts.lending_vault.to_account_info();

    let la = &mut ctx.accounts.loan_account;
    la.borrower         = ctx.accounts.borrower.key();
    la.collateral_vault = ctx.accounts.collateral_vault.key();
    la.principal_usd    = amount;
    la.fee_usd          = fee;
    la.opened_at        = clock.unix_timestamp;
    la.expires_at       = clock.unix_timestamp + LoanAccount::DURATION_SECS;
    la.status           = LoanStatus::Active;
    la.bump             = ctx.bumps.loan_account;

    let lv = &mut ctx.accounts.lending_vault;
    lv.available_liquidity     -= amount;
    lv.total_loans_outstanding += amount;

    let seeds: &[&[u8]] = &[b"lending_vault", &[vault_bump]];
    let signer = &[seeds];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.vault_usdc_ata.to_account_info(),
                to:        ctx.accounts.borrower_usdc_ata.to_account_info(),
                authority: vault_info,
            },
            signer,
        ),
        amount,
    )
}
