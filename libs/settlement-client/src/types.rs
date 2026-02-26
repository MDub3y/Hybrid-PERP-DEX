use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SettlementError {
    #[error("ENGINE_SIGNING_KEY not found in environment")]
    MissingKeypair,
    #[error("Invalid keypair format")]
    InvalidKeypair,
    #[error("Failed to sign trade: {0}")]
    SigningError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSettlementMessage {
    pub trade_id: u64,
    pub buyer: [u8; 32],
    pub seller: [u8; 32],
    pub market: [u8; 16],
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTradeSettlement {
    pub signature: Vec<u8>,
    pub buyer_nonce: u64,
    pub seller_nonce: u64,
}