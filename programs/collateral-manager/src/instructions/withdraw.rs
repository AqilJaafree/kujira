use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};
use crate::state::CollateralVault;
use crate::error::CollateralError;

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds  = [b"collateral_vault", owner.key().as_ref(), collateral_vault.lp_position_mint.as_ref()],
        bump   = collateral_vault.bump,
        constraint = collateral_vault.owner == owner.key(),
        constraint = collateral_vault.active_loan.is_none() @ CollateralError::ActiveLoanExists,
        close  = owner,
    )]
    pub collateral_vault: Account<'info, CollateralVault>,

    #[account(
        mut,
        seeds = [b"vault_custody", collateral_vault.key().as_ref()],
        bump,
        token::mint      = collateral_vault.lp_position_mint,
        token::authority = collateral_vault,
    )]
    pub custody_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = owner_lp_ata.owner == owner.key(),
        constraint = owner_lp_ata.mint  == collateral_vault.lp_position_mint,
    )]
    pub owner_lp_ata: Account<'info, TokenAccount>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<WithdrawCollateral>) -> Result<()> {
    let v = &ctx.accounts.collateral_vault;
    let seeds: &[&[u8]] = &[
        b"collateral_vault",
        v.owner.as_ref(),
        v.lp_position_mint.as_ref(),
        &[v.bump],
    ];
    let signer = &[seeds];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.custody_ata.to_account_info(),
                to:        ctx.accounts.owner_lp_ata.to_account_info(),
                authority: ctx.accounts.collateral_vault.to_account_info(),
            },
            signer,
        ),
        1,
    )?;

    token::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account:     ctx.accounts.custody_ata.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority:   ctx.accounts.collateral_vault.to_account_info(),
        },
        signer,
    ))
}
