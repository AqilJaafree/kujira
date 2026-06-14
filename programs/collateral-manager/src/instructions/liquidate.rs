use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};
use crate::state::CollateralVault;
use crate::error::CollateralError;

#[derive(Accounts)]
pub struct TriggerLiquidation<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collateral_vault", collateral_vault.owner.as_ref(), collateral_vault.lp_position_mint.as_ref()],
        bump  = collateral_vault.bump,
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
        constraint = liquidator_ata.owner == liquidator.key(),
        constraint = liquidator_ata.mint  == collateral_vault.lp_position_mint,
    )]
    pub liquidator_ata: Account<'info, TokenAccount>,

    pub token_program:  Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<TriggerLiquidation>, outstanding_loan_usd: u64) -> Result<()> {
    // Snapshot all account_infos before mutable borrow of vault.
    let vault_info      = ctx.accounts.collateral_vault.to_account_info();
    let custody_info    = ctx.accounts.custody_ata.to_account_info();
    let liquidator_ata  = ctx.accounts.liquidator_ata.to_account_info();
    let liquidator_info = ctx.accounts.liquidator.to_account_info();
    let token_prog      = ctx.accounts.token_program.to_account_info();

    let v = &mut ctx.accounts.collateral_vault;
    require!(v.is_liquidatable(outstanding_loan_usd), CollateralError::LtvHealthy);

    let owner_key = v.owner;
    let mint_key  = v.lp_position_mint;
    let bump      = v.bump;

    // Update state before CPIs (PDA is the authority, immutable after this).
    v.active_loan          = None;
    v.collateral_usd_value = 0;
    v.max_borrow_usd       = 0;

    let seeds: &[&[u8]] = &[b"collateral_vault", owner_key.as_ref(), mint_key.as_ref(), &[bump]];
    let signer = &[seeds];

    token::transfer(
        CpiContext::new_with_signer(
            token_prog.clone(),
            Transfer { from: custody_info.clone(), to: liquidator_ata, authority: vault_info.clone() },
            signer,
        ),
        1,
    )?;

    token::close_account(CpiContext::new_with_signer(
        token_prog,
        CloseAccount { account: custody_info, destination: liquidator_info, authority: vault_info },
        signer,
    ))
}
