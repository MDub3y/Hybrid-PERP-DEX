use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 128, seeds = [b"engine_config"], bump)]
    pub config: Account<'info, EngineConfig>,
    pub usdc_mint: AccountInfo<'info>, // Simplified for prototype
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMarginAccount<'info> {
    #[account(init, payer = owner, space = 8 + 1024, seeds = [b"margin_account", owner.key().as_ref()], bump)]
    pub margin_account: Account<'info, MarginAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_margin_account(ctx: Context<CreateMarginAccount>) -> Result<()> {
    let acc = &mut ctx.accounts.margin_account;
    acc.owner = ctx.accounts.owner.key();
    acc.nonce = 0;
    acc.bump = ctx.bumps.margin_account;
    Ok(())
}