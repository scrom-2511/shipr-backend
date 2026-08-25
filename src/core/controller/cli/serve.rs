use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use futures::lock::Mutex;

use crate::{
    app::{db::DbPool, state::AppState},
    app_errors::AppError,
    core::controller::{
        api::vm_request_proxy::VmRequestProxy,
        cli::listen_idle_kill::listen_idle_kill,
        dispatcher::job_dispatcher::JobDispatcher,
        queue::{idle_kill_queue::IdleKillQueue, lapin::Lapin},
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
    state: web::Data<AppState>,
) -> Result<(), AppError> {
    let lapin_conn = Lapin::new().await?;
    let idle_kill_queue = IdleKillQueue::new(&lapin_conn).await?;
    let idle_kill_queue_data = web::Data::new(idle_kill_queue.clone());

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
        pool.clone(),
        redis.clone(),
        idle_kill_queue,
        state,
    )?));

    // Spawn idle kill listener worker task
    {
        let vm_pool = vm_pool.clone();
        let id_allocator = id_allocator.clone();
        let pool_data = web::Data::new(pool.clone());
        let idle_kill_queue_data = idle_kill_queue_data.clone();

        tokio::spawn(async move {
            listen_idle_kill(
                idle_kill_queue_data,
                vm_pool,
                id_allocator,
                pool_data,
                redis,
            )
            .await;
        });
    }

    for _ in 0..1 {
        let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
        new_vm.create_hot_vm(&vm_pool).await?;
    }

    println!("Starting proxy server");

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
