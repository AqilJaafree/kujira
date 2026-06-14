use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::LendingVault;

#[derive(Accounts)]
pub struct DepositVaultLiquidity<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(mut, seeds = [b"lending_vault"], bump = lending_vault.bump)]
    pub lending_vault: Account<'info, LendingVault>,

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
        constraint = depositor_usdc_ata.owner == depositor.key(),
        constraint = depositor_usdc_ata.mint  == lending_vault.usdc_mint,
    )]
    pub depositor_usdc_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<DepositVaultLiquidity>, amount: u64) -> Result<()> {
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.depositor_usdc_ata.to_account_info(),
                to:        ctx.accounts.vault_usdc_ata.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount,
    )?;
    let v = &mut ctx.accounts.lending_vault;
    v.total_liquidity     += amount;
    v.available_liquidity += amount;
    Ok(())
}
