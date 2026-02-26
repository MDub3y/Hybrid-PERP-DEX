use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"margin_account", owner.key().as_ref()], bump = margin_account.bump)]
    pub margin_account: Account<'info, MarginAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

pub fn deposit_handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    ctx.accounts.margin_account.collateral += amount;
    Ok(())
}