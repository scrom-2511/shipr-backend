use std::collections::HashMap;

use actix_web::{
    App, HttpServer,
    web::{self},
};
use tokio::sync::{Mutex, broadcast::Sender};

use crate::{
    app_errors::AppError,
    core::app_types::LogsStore,
    core::controller::{
        cli::{
            listen_deploy::listen_deploy, listen_idle_kill::listen_idle_kill,
            listen_redeploy::listen_redeploy,
        },
        dispatcher::job_dispatcher::JobDispatcher,
        handlers::{
            deploy::deploy_handler,
            kill_vm::kill_vm_handler,
            logs::{logs_handler, stream_logs_handler},
            redeployment_completed::redeploy_completed_handler,
        },
        queue::{
            deploy_queue::DeployQueue, idle_kill_queue::IdleKillQueue, lapin::Lapin,
            redeploy_queue::ReDeployQueue,
        },
        storage::{redis::Redis, s3::S3Service},
        vm::{
            firecracker::Firecracker, heartbeat_store::HeartbeatStore, id_allocator::IdAllocator,
            vm_pool::VmPool,
        },
    },
};

use crate::app::db::DbPool;

pub async fn listen(
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    s3_service: S3Service,
    heartbeat_store: HeartbeatStore,
    pool: DbPool,
    redis: Redis,
) -> Result<(), AppError> {
    let logs_store: LogsStore =
        web::Data::new(Mutex::new(HashMap::<String, Sender<String>>::new()));

    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
        pool.clone(),
    );

    for _ in 0..1 {
        let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
        new_vm.create_hot_vm(&vm_pool).await?;
    }

    let lapin_conn = Lapin::new().await?;
    let deploy_queue = web::Data::new(DeployQueue::new(&lapin_conn).await?);
    let redeploy_queue = web::Data::new(ReDeployQueue::new(&lapin_conn).await?);
    let idle_kill_queue = web::Data::new(IdleKillQueue::new(&lapin_conn).await?);

    let s3_service = s3_service.clone();

    println!("Queues created");

    {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();
        let deploy_queue = deploy_queue.clone();
        let job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();

        tokio::spawn(async move {
            listen_deploy(
                s3_service,
                job_dispatcher,
                id_allocator,
                vm_pool,
                deploy_queue,
            )
            .await
        });
    }

    {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();
        let redeploy_queue = redeploy_queue.clone();
        let job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();
        let pool_data = web::Data::new(pool.clone());

        tokio::spawn(async move {
            listen_redeploy(
                s3_service,
                job_dispatcher,
                id_allocator,
                vm_pool,
                redeploy_queue,
                pool_data,
            )
            .await;
        });
    }

    {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();
        let idle_kill_queue = idle_kill_queue.clone();
        let redis = redis.clone();
        let pool_data = web::Data::new(pool.clone());

        tokio::spawn(async move {
            listen_idle_kill(
                idle_kill_queue,
                vm_pool,
                id_allocator,
                pool_data,
                web::Data::new(redis),
            )
            .await;
        });
    }

    let id_allocator = web::Data::new(id_allocator);
    let vm_pool = web::Data::new(vm_pool);

    HttpServer::new(move || {
        App::new()
            .app_data(deploy_queue.clone())
            .app_data(redeploy_queue.clone())
            .app_data(idle_kill_queue.clone())
            .app_data(id_allocator.clone())
            .app_data(vm_pool.clone())
            .app_data(logs_store.clone())
            .route("/kill-vm", web::post().to(kill_vm_handler))
            .route("/deploy", web::post().to(deploy_handler))
            .route("/send-logs", web::post().to(logs_handler))
            .route("/logs/{project_id}", web::get().to(stream_logs_handler))
            .route(
                "/redeploy-completed",
                web::post().to(redeploy_completed_handler),
            )
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await?;

    Ok(())
}
