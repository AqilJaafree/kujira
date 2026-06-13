use anchor_lang::prelude::*;

declare_id!("5Ymdq7VQKyhnerpYBaoc3pgV3CYdXHhGbBWQTVwf463k");

#[program]
pub mod settlement {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
