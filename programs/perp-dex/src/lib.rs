use anchor_lang::prelude::*;
pub mod instructions;
pub mod state;
pub mod error;

use instructions::*;

declare_id!("PERPdex111111111111111111111111111111111111");

#[program]
pub mod perp_dex {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, engine_signer: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.engine_signer = engine_signer;
        config.authority = ctx.accounts.authority.key();
        config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn create_margin_account(ctx: Context<CreateMarginAccount>) -> Result<()> {
        instructions::initialize::create_margin_account(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::deposit_handler(ctx, amount)
    }

    pub fn settle_trade(
        ctx: Context<SettleTrade>,
        trade_id: u64,
        price: u64,
        quantity: u64,
        buyer_nonce: u64,
        seller_nonce: u64,
    ) -> Result<()> {
        instructions::settle_trade::settle_trade_handler(ctx, trade_id, price, quantity, buyer_nonce, seller_nonce)
    }
}