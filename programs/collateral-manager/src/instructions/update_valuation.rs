use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateValuation<'info> {
    pub system_program: Program<'info, System>,
}
pub fn handler(_ctx: Context<UpdateValuation>, _usd_value: u64) -> Result<()> { Ok(()) }
