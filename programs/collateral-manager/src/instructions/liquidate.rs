use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct TriggerLiquidation<'info> {
    pub system_program: Program<'info, System>,
}
pub fn handler(_ctx: Context<TriggerLiquidation>, _outstanding_loan_usd: u64) -> Result<()> { Ok(()) }
