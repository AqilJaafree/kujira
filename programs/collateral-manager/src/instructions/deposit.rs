use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{CollateralVault, LpProtocol};
use crate::error::CollateralError;

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = CollateralVault::LEN,
        seeds = [b"collateral_vault", owner.key().as_ref(), lp_position_mint.key().as_ref()],
        bump,
    )]
    pub collateral_vault: Account<'info, CollateralVault>,

    /// CHECK: identity verified via token transfer mint constraint below
    pub lp_position_mint: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = owner_lp_ata.owner == owner.key(),
        constraint = owner_lp_ata.mint == lp_position_mint.key(),
        constraint = owner_lp_ata.amount >= 1 @ CollateralError::InsufficientLpBalance,
    )]
    pub owner_lp_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = owner,
        seeds = [b"vault_custody", collateral_vault.key().as_ref()],
        bump,
        token::mint      = lp_position_mint,
        token::authority = collateral_vault,
    )]
    pub custody_ata: Account<'info, TokenAccount>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent:           Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<DepositCollateral>, lp_protocol: LpProtocol) -> Result<()> {
    let clock = Clock::get()?;
    let v = &mut ctx.accounts.collateral_vault;
    v.owner                = ctx.accounts.owner.key();
    v.lp_position_mint     = ctx.accounts.lp_position_mint.key();
    v.lp_protocol          = lp_protocol;
    v.collateral_usd_value = 0;
    v.max_borrow_usd       = 0;
    v.active_loan          = None;
    v.deposited_at         = clock.unix_timestamp;
    v.bump                 = ctx.bumps.collateral_vault;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.owner_lp_ata.to_account_info(),
                to:        ctx.accounts.custody_ata.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        1,
    )
}
