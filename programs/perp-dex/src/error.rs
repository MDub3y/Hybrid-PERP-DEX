use anchor_lang::prelude::*;

#[error_code]
pub enum PerpError {
    #[msg("Signature verification failed: missing Ed25519 instruction")]
    MissingSignature,
    #[msg("Invalid signature program ID")]
    InvalidSignatureProgram,
    #[msg("Account nonce is stale")]
    StaleNonce,
    #[msg("Price is out of bounds")]
    PriceOutOfBounds,
    #[msg("Zero position size")]
    ZeroPositionSize,
    #[msg("Insufficient collateral")]
    InsufficientCollateral,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Max positions reached")]
    MaxPositionsReached,
    #[msg("Trade message mismatch")]
    TradeMessageMismatch,
}