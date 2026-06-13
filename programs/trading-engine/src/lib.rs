use anchor_lang::prelude::*;

declare_id!("FVpeDwQgf7GSz8CQachQXzZZJrKE9sJ2FWbuoHLuQkDs");

#[program]
pub mod trading_engine {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
