use shipr::core::controller::storage::redis::Redis;
use shipr::core::controller::vm::id_allocator::IdAllocator;
use shipr::core::controller::vm::vm_pool::VmPool;
use shipr::dns::server::ShiprDNS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let redis = Redis::new().await;
    let id_allocator = IdAllocator::new(redis.clone());
    let vm_pool = VmPool::new(redis.clone(), id_allocator.clone());
    let dns = ShiprDNS::new(vm_pool);

    println!("starting...");

    dns.start().await?;

    Ok(())
}
