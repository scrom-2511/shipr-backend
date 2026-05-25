use shipr::core::controller::{
    cli::cli::cli,
    storage::{redis::Redis, s3::S3Service},
    vm::{heartbeat_store::HeartbeatStore, id_allocator::IdAllocator, vm_pool::VmPool},
};

pub mod worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis = Redis::new();
    let id_allocator = IdAllocator::new(redis.clone());
    let vm_pool = VmPool::new(redis.clone(), id_allocator.clone());
    let s3_service = S3Service::new().await;
    let heartbeat_store = HeartbeatStore::new(redis);

    cli(vm_pool, id_allocator, s3_service, heartbeat_store).await?;

    Ok(())
}
