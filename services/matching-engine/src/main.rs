use std::collections::{BTreeMap, VecDeque};
use common_utils::{Order, MatchResult};
use fred::prelude::*;
use rust_decimal::Decimal;
use std::env;
use dotenvy::dotenv;
use anyhow::Result;

struct Engine {
    bids: BTreeMap<Decimal, VecDeque<Order>>,
    asks: BTreeMap<Decimal, VecDeque<Order>>,
    trade_counter: u64,
}

impl Engine {
    async fn match_order(&mut self, order: Order, redis: &RedisClient) {
        if order.side == "BUY" {
            self.process_buy(order, redis).await;
        } else {
            self.process_sell(order, redis).await;
        }
    }

    async fn process_buy(&mut self, mut buy_order: Order, redis: &RedisClient) {
        while let Some((&price, orders)) = self.asks.iter_mut().next() {
            if price > buy_order.price { break; }
            while let Some(mut ask) = orders.pop_front() {
                let fill_qty = buy_order.quantity.min(ask.quantity);
                self.trade_counter += 1;
                let res = MatchResult {
                    trade_id: self.trade_counter,
                    price,
                    quantity: fill_qty,
                    buyer_id: buy_order.user_id.clone(),
                    seller_id: ask.user_id.clone(),
                };

                Self::broadcast_match(res, redis).await;

                buy_order.quantity -= fill_qty;
                if !ask.quantity.is_zero() { orders.push_front(ask); }
                if buy_order.quantity.is_zero() { break; }
            }
            if orders.is_empty() { self.asks.remove(&price); }
            if buy_order.quantity.is_zero() { return; }
        }
        self.bids.entry(buy_order.price).or_default().push_back(buy_order);
    }

    async fn process_sell(&mut self, mut sell_order: Order, redis: &RedisClient) {
        while let Some((&price, orders)) = self.bids.iter_mut().next_back() {
            if price < sell_order.price { break; }
            while let Some(mut bid) = orders.pop_front() {
                let fill_qty = sell_order.quantity.min(bid.quantity);
                self.trade_counter += 1;
                let res = MatchResult {
                    trade_id: self.trade_counter,
                    price,
                    quantity: fill_qty,
                    buyer_id: bid.user_id.clone(),
                    seller_id: sell_order.user_id.clone(),
                };

                Self::broadcast_match(res, redis).await;

                sell_order.quantity -= fill_qty;
                if !bid.quantity.is_zero() { orders.push_front(bid); }
                if sell_order.quantity.is_zero() { break; }
            }
            if orders.is_empty() { self.bids.remove(&price); }
            if sell_order.quantity.is_zero() { return; }
        }
        self.asks.entry(sell_order.price).or_default().push_back(sell_order);
    }

    // FIX: Removed &self to avoid borrow checker error
    async fn broadcast_match(res: MatchResult, redis: &RedisClient) {
        if let Ok(payload) = serde_json::to_string(&res) {
            let _ = redis.lpush::<i64, _, _>("SETTLEMENT_QUEUE", payload.clone()).await;
            let _ = redis.lpush::<i64, _, _>("DB_QUEUE", payload).await;
            println!("🎯 Match Found: Trade #{}", res.trade_id);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
    let mut engine = Engine { bids: BTreeMap::new(), asks: BTreeMap::new(), trade_counter: 0 };
    let config = RedisConfig::from_url(&redis_url)?;
    let client = Builder::from_config(config).build()?;
    client.init().await?;
    loop {
        if let Ok(Some(data)) = client.rpop::<Option<String>, _>("ORDER_QUEUE", None).await {
            if let Ok(order) = serde_json::from_str::<Order>(&data) {
                engine.match_order(order, &client).await;
            }
        }
    }
}