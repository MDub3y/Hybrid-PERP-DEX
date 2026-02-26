use num_traits::ToPrimitive;
use tokio_postgres::{NoTls, types::ToSql};
use fred::prelude::*;
use common_utils::MatchResult;
use std::env;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Setup Environment
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());

    // 2. Connect to Postgres
    // tokio-postgres requires spawning the connection task separately
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("❌ Postgres connection error: {}", e);
        }
    });

    println!("✅ Connected to Postgres!");

    // 3. Initialize Table (Self-Healing Schema)
    // We use BIGINT (i64) for price/qty to store 6-decimal fixed point values.
    // This is a high-frequency trading standard to avoid NUMERIC overhead.
    client.batch_execute("
        CREATE TABLE IF NOT EXISTS trades (
            trade_id BIGINT PRIMARY KEY,
            market VARCHAR(16) NOT NULL,
            buyer_id VARCHAR(44) NOT NULL,
            seller_id VARCHAR(44) NOT NULL,
            price BIGINT NOT NULL,
            quantity BIGINT NOT NULL,
            timestamp BIGINT NOT NULL
        );
    ").await?;
    println!("📊 Database schema verified.");

    // 4. Initialize Redis
    let config = RedisConfig::from_url(&redis_url)?;
    let redis = Builder::from_config(config).build()?;
    redis.init().await?;
    println!("🚀 Historian is watching DB_QUEUE...");

    loop {
        // Pop from Redis
        match redis.rpop::<Option<String>, _>("DB_QUEUE", None).await {
            Ok(Some(raw_data)) => {
                let m: MatchResult = match serde_json::from_str(&raw_data) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        eprintln!("❌ Failed to parse match data: {}", e);
                        continue;
                    }
                };

                // Convert Decimal to i64 (multiply by 1,000,000 for 6 decimal places)
                let p_i64 = (m.price * rust_decimal::Decimal::from(1_000_000))
                    .to_i64()
                    .unwrap_or(0);
                let q_i64 = (m.quantity * rust_decimal::Decimal::from(1_000_000))
                    .to_i64()
                    .unwrap_or(0);

                let market = "SOL_USDC";
                let trade_id_i64 = m.trade_id as i64;
                let ts_millis = chrono::Utc::now().timestamp_millis();
                
                // Use an array of trait objects to mix types
                let params: &[&(dyn ToSql + Sync)] = &[
                    &trade_id_i64, 
                    &market, 
                    &m.buyer_id, 
                    &m.seller_id, 
                    &p_i64, 
                    &q_i64, 
                    &ts_millis
                ];
                
                let res = client.execute(
                    "INSERT INTO trades (trade_id, market, buyer_id, seller_id, price, quantity, timestamp) 
                        VALUES ($1, $2, $3, $4, $5, $6, $7) 
                        ON CONFLICT (trade_id) DO NOTHING",
                    params
                ).await;

                match res {
                    Ok(_) => println!("💾 Persisted Trade #{}", m.trade_id),
                    Err(e) => eprintln!("❌ Database insert error: {}", e),
                }
            }
            Ok(None) => {
                // Queue empty, wait 100ms
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Err(e) => {
                eprintln!("❌ Redis error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}