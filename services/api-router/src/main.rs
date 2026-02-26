use actix_web::{web, App, HttpResponse, HttpServer, post, Responder};
use common_utils::{Order, OrderRequest};
use fred::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::env;

#[post("/order")]
async fn create_order(redis: web::Data<RedisClient>, req: web::Json<OrderRequest>) -> impl Responder {
    // 1. Transform OrderRequest (Strings) -> Order (Decimals)
    let order = match (Decimal::from_str(&req.price), Decimal::from_str(&req.quantity)) {
        (Ok(p), Ok(q)) => Order {
            user_id: req.user_id.clone(),
            price: p,
            quantity: q,
            side: req.side.clone(),
        },
        _ => return HttpResponse::BadRequest().body("Invalid price or quantity format"),
    };

    // 2. Serialize and push to the queue the Engine is actually watching
    let payload = serde_json::to_string(&order).unwrap();
    
    if let Err(e) = redis.lpush::<i64, _, _>("ORDER_QUEUE", payload).await {
        return HttpResponse::InternalServerError().body(e.to_string());
    }

    println!("✅ Order Queued: {} {} @ {}", order.side, order.quantity, order.price);
    HttpResponse::Accepted().json(serde_json::json!({"status": "queued"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1:6379".into());

    let config = RedisConfig::from_url(&redis_url).unwrap();
    let redis = Builder::from_config(config).build().unwrap();
    redis.init().await.unwrap();

    println!("🚀 API Router running on 127.0.0.1:7000");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(redis.clone()))
            .service(create_order)
    })
    .bind("127.0.0.1:7000")?
    .run()
    .await
}