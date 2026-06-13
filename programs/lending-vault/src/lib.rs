use anchor_lang::prelude::*;

declare_id!("4gFZQRU4Qz69AsEBwoHacd5wA88andm1eMSkNzK3MsAJ");

#[program]
pub mod lending_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
