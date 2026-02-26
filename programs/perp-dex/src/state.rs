use anchor_lang::prelude::*;

pub const MAX_POSITIONS: usize = 8;
pub const MARKET_NAME_LEN: usize = 16;

#[account]
pub struct EngineConfig {
    pub authority: Pubkey,
    pub engine_signer: Pubkey,
    pub usdc_mint: Pubkey,
    pub maintenance_margin_bps: u16,
    pub bump: u8,
}

#[account]
pub struct MarginAccount {
    pub owner: Pubkey,
    pub collateral: u64,
    pub positions: [Position; MAX_POSITIONS],
    pub position_count: u8,
    pub nonce: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct Position {
    pub market: [u8; MARKET_NAME_LEN],
    pub size: i64,      // signed + for long or short
    pub avg_entry_price: u64,
}