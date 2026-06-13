use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    pub system_program: Program<'info, System>,
}
pub fn handler(_ctx: Context<WithdrawCollateral>) -> Result<()> { Ok(()) }
