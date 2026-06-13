use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("6bcDZNzrSKJE5eA4HBRryqweVy3Dq4YeXfpqwG9VEe3f");

#[program]
pub mod collateral_manager {
    use super::*;

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        lp_protocol: state::LpProtocol,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, lp_protocol)
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>) -> Result<()> {
        instructions::withdraw::handler(ctx)
    }

    pub fn update_valuation(ctx: Context<UpdateValuation>, usd_value: u64) -> Result<()> {
        instructions::update_valuation::handler(ctx, usd_value)
    }

    pub fn trigger_liquidation(
        ctx: Context<TriggerLiquidation>,
        outstanding_loan_usd: u64,
    ) -> Result<()> {
        instructions::liquidate::handler(ctx, outstanding_loan_usd)
    }
}
