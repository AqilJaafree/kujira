use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::LendingVault;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = LendingVault::LEN,
        seeds = [b"lending_vault"],
        bump,
    )]
    pub lending_vault: Account<'info, LendingVault>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [b"vault_usdc", lending_vault.key().as_ref()],
        bump,
        token::mint      = usdc_mint,
        token::authority = lending_vault,
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent:           Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializeVault>) -> Result<()> {
    let v = &mut ctx.accounts.lending_vault;
    v.authority               = ctx.accounts.authority.key();
    v.usdc_mint               = ctx.accounts.usdc_mint.key();
    v.total_liquidity         = 0;
    v.available_liquidity     = 0;
    v.total_loans_outstanding = 0;
    v.borrow_fee_bps          = 10; // 0.1%
    v.bump                    = ctx.bumps.lending_vault;
    Ok(())
}
