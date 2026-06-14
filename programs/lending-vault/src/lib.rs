use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("4gFZQRU4Qz69AsEBwoHacd5wA88andm1eMSkNzK3MsAJ");

#[program]
pub mod lending_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn deposit_vault_liquidity(ctx: Context<DepositVaultLiquidity>, amount: u64) -> Result<()> {
        instructions::deposit_liquidity::handler(ctx, amount)
    }

    pub fn issue_loan(ctx: Context<IssueLoan>, amount: u64) -> Result<()> {
        instructions::issue_loan::handler(ctx, amount)
    }

    pub fn repay_loan(ctx: Context<RepayLoan>, proceeds: u64) -> Result<()> {
        instructions::repay_loan::handler(ctx, proceeds)
    }

    pub fn force_expire_loan(ctx: Context<ForceExpireLoan>) -> Result<()> {
        instructions::repay_loan::force_expire_handler(ctx)
    }
}
