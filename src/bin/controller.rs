use std::sync::Arc;

use actix_web::web;
use dodopayments::Client;
use shipr::{
    app::state::AppState,
    core::controller::{
        cli::cli::cli,
        storage::{redis::Redis, s3::S3Service},
        vm::{heartbeat_store::HeartbeatStore, id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

pub mod worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let db_url = "postgresql://neondb_owner:npg_RlYICzb47Sps@ep-weathered-mode-aqn5uvc3-pooler.c-8.us-east-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require";
    let pool = sea_orm::Database::connect(db_url).await?;

    let redis = Redis::new().await;
    let id_allocator = IdAllocator::new(redis.clone());
    let vm_pool = VmPool::new(redis.clone(), id_allocator.clone());
    let s3_service = S3Service::new().await;
    let heartbeat_store = HeartbeatStore::new(redis.clone());
    let product_id =
        std::env::var("DODO_PRODUCT_ID").unwrap_or_else(|_| "pdt_compute_credits".to_string());

    let client = Client::from_env()?;
    let app_state = web::Data::new(AppState {
        db: pool.clone(),
        client: Arc::new(client),
        product_id,
    });

    cli(
        vm_pool,
        id_allocator,
        s3_service,
        heartbeat_store,
        pool,
        redis,
        app_state,
    )
    .await?;

    Ok(())
}
