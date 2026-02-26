use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub user_id: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub user_id: String,
    pub price: String,
    pub quantity: String,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub trade_id: u64,
    pub price: Decimal,
    pub quantity: Decimal,
    pub buyer_id: String,
    pub seller_id: String,
}