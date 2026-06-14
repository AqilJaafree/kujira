use anchor_lang::prelude::*;
use crate::state::CollateralVault;

#[derive(Accounts)]
pub struct UpdateValuation<'info> {
    // Any signer may call (keeper/oracle bot).
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"collateral_vault", collateral_vault.owner.as_ref(), collateral_vault.lp_position_mint.as_ref()],
        bump  = collateral_vault.bump,
    )]
    pub collateral_vault: Account<'info, CollateralVault>,
}

pub fn handler(ctx: Context<UpdateValuation>, usd_value: u64) -> Result<()> {
    let v = &mut ctx.accounts.collateral_vault;
    v.collateral_usd_value = usd_value;
    v.max_borrow_usd       = CollateralVault::max_borrow_from(usd_value);
    Ok(())
}
