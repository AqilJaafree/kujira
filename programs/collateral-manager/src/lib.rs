use anchor_lang::prelude::*;

declare_id!("6bcDZNzrSKJE5eA4HBRryqweVy3Dq4YeXfpqwG9VEe3f");

#[program]
pub mod collateral_manager {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
