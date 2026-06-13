use anchor_lang::prelude::*;

// Stub — implemented in Task 3
#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    pub system_program: Program<'info, System>,
}
pub fn handler(_ctx: Context<DepositCollateral>, _lp_protocol: crate::state::LpProtocol) -> Result<()> {
    Ok(())
}
