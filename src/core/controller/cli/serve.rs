use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use futures::lock::Mutex;

use crate::{
    app::db::DbPool,
    app_errors::AppError,
    core::controller::{
        api::vm_request_proxy::VmRequestProxy,
        dispatcher::job_dispatcher::JobDispatcher,
        storage::{redis::Redis, s3::S3Service},
        vm::{
            firecracker::Firecracker, heartbeat_store::HeartbeatStore, id_allocator::IdAllocator,
            vm_pool::VmPool,
        },
    },
};

pub async fn proxy(
    vm_request_proxy: web::Data<Mutex<VmRequestProxy>>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    vm_request_proxy.lock().await.proxy_request(req, body).await
}

pub async fn serve(
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    s3_service: S3Service,
    heartbeat_store: HeartbeatStore,
    pool: DbPool,
    redis: Redis,
) -> Result<(), AppError> {
    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
        pool.clone(),
    );

    let vm_request_proxy = web::Data::new(Mutex::new(VmRequestProxy::new(
        vm_pool.clone(),
        job_dispatcher.clone(),
        heartbeat_store.clone(),
        pool,
        redis,
    )?));

    for _ in 0..1 {
        let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
        new_vm.create_hot_vm(&vm_pool).await?;
    }

    println!("Starting server");

    let vm_pool_dns = vm_pool.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::dns::server::start_dns_server(vm_pool_dns).await {
            eprintln!("DNS Server error: {}", e);
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(vm_request_proxy.clone())
            .default_service(web::to(proxy))
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await?;

    Ok(())
}
