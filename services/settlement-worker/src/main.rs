use settlement_client::{SettlementClient, signer::EngineSigner};
use fred::prelude::*;
use common_utils::MatchResult;
use solana_sdk::pubkey::Pubkey;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::str::FromStr;
use anyhow::Result;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Infrastructure
    dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());
    let rpc_url = std::env::var("SOLANA_RPC_URL").unwrap_or("http://127.0.0.1:8899".into());
    let program_id = Pubkey::from_str(&std::env::var("PROGRAM_ID")?)?;

    // Initialize Redis
    let config = RedisConfig::from_url(&redis_url)?;
    let redis = Builder::from_config(config).build()?;
    redis.init().await?;
    
    // Initialize Solana Client
    // We assume the Relayer Key is stored as a hex string in the env
    let relayer_key_hex = std::env::var("RELAYER_KEYPAIR_HEX")?;
    let relayer_bytes = hex::decode(relayer_key_hex)?;
    let relayer_fee_payer = Keypair::from_bytes(&relayer_bytes)?;

    let client = SettlementClient {
        rpc: RpcClient::new(rpc_url),
        relayer_fee_payer,
    };

    let engine_signer = EngineSigner::from_env()?;

    println!("🚀 Settlement Relayer is live. Watching SETTLEMENT_QUEUE...");

    loop {
        // 2. Pop match from Redis
        if let Ok(Some(data)) = redis.rpop::<Option<String>, _>("SETTLEMENT_QUEUE", None).await {
            let m: MatchResult = match serde_json::from_str(&data) {
                Ok(parsed) => parsed,
                Err(e) => {
                    eprintln!("❌ Failed to parse match: {}", e);
                    continue;
                }
            };

            println!("🔄 Processing Trade #{}...", m.trade_id);

            // 3. Settle on Solana
            // The client now handles: PDA derivation, msg reconstruction, signing, and broadcasting.
            match client.settle_trade(&m, &engine_signer, &program_id).await {
                Ok(tx_sig) => {
                    println!("✅ Trade {} Settled! TX: {}", m.trade_id, tx_sig);
                }
                Err(e) => {
                    eprintln!("❌ Settlement error for trade {}: {:?}", m.trade_id, e);
                    // Re-queue for retry (optional)
                    let _ = redis.lpush::<i64, _, _>("SETTLEMENT_QUEUE", data).await;
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        } else {
            // Idle wait
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}