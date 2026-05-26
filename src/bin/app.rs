use std::error::Error;

use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use shipr::app::controllers::auth::github_signup::{github_auth_url, github_callback};
use shipr::app::controllers::auth::signin::signin_controller;
use shipr::app::controllers::auth::signup::signup_controller;
use shipr::app::controllers::github::get_state::get_state;
use shipr::app::controllers::github::update_userid_github_app_installations::update_userid_github_app_installations;
use shipr::app::controllers::project::add_new_project::add_new_project;
use shipr::app::controllers::project::check_name_availability::check_repo_name_availability;
use shipr::app::controllers::project::deploy_project::deploy_project_controller;
use shipr::app::controllers::project::get_all_deployed_projects::get_all_deployed_projects_controller;
use shipr::app::controllers::project::get_all_github_app_installed_repos::get_all_github_app_installed_repos;
use shipr::app::controllers::project::get_project_details::get_project_details_controller;
use shipr::app::controllers::project::job_completed::job_completed_controller;

use shipr::app::middlewares::is_logged_in::is_logged_in;
use shipr::app::webhooks::github_installation::github_webhook_installation;
use shipr::core::controller::cli::listen_deploy::listen_deploy;
use shipr::core::controller::cli::listen_redeploy::listen_redeploy;
use shipr::core::controller::dispatcher::job_dispatcher::JobDispatcher;
use shipr::core::controller::queue::deploy_queue::DeployQueue;
use shipr::core::controller::queue::lapin::Lapin;
use shipr::core::controller::queue::redeploy_queue::ReDeployQueue;
use shipr::core::controller::storage::redis::Redis;
use shipr::core::controller::storage::s3::S3Service;
use shipr::core::controller::vm::firecracker::Firecracker;
use shipr::core::controller::vm::heartbeat_store::HeartbeatStore;
use shipr::core::controller::vm::id_allocator::IdAllocator;
use shipr::core::controller::vm::vm_pool::VmPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let db_url = "postgresql://neondb_owner:npg_RlYICzb47Sps@ep-weathered-mode-aqn5uvc3-pooler.c-8.us-east-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require";

    println!("{}", db_url);

    let pool = sqlx::postgres::PgPool::connect(db_url).await?;

    println!("Connected to database");

    sqlx::migrate!("src/app/migrations").run(&pool).await?;

    println!("Migrations applied");

    println!("Server running on port 9000");

    let redis = Redis::new();
    let id_allocator = IdAllocator::new(redis.clone());
    let vm_pool = VmPool::new(redis.clone(), id_allocator.clone());
    let s3_service = S3Service::new().await;
    let heartbeat_store = HeartbeatStore::new(redis);

    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
    );

    for _ in 0..1 {
        let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
        new_vm.create_new_vm_and_add_to_pool(&vm_pool).await?;
    }

    let lapin_conn = Lapin::new().await?;
    let deploy_queue = web::Data::new(DeployQueue::new(&lapin_conn).await?);
    let redeploy_queue = web::Data::new(ReDeployQueue::new(&lapin_conn).await?);

    let s3_service = s3_service.clone();

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

        tokio::spawn(async move {
            listen_redeploy(
                s3_service,
                job_dispatcher,
                id_allocator,
                vm_pool,
                redeploy_queue,
            )
            .await;
        });
    }

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allowed_origin("https://terminology-club-trader-domain.trycloudflare.com")
            .allowed_origin("http://localhost:5173")
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(deploy_queue.clone())
            .app_data(redeploy_queue.clone())
            .app_data(id_allocator.clone())
            .app_data(vm_pool.clone())
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .route("/signup", web::post().to(signup_controller))
            .route("/signin", web::post().to(signin_controller))
            .route("/add-project", web::post().to(add_new_project))
            .route("/auth/github", web::get().to(github_auth_url))
            .route("/auth/github/callback", web::get().to(github_callback))
            .route(
                "/webhook/github",
                web::post().to(github_webhook_installation),
            )
            .route(
                "/check-repo-name-availability",
                web::post().to(check_repo_name_availability),
            )
            .route("/job-completed", web::post().to(job_completed_controller))
            .service(
                web::scope("")
                    .wrap(from_fn(is_logged_in))
                    .route("/get-state", web::get().to(get_state))
                    .route(
                        "/github/update-userid-github-app-installations",
                        web::post().to(update_userid_github_app_installations),
                    )
                    .route(
                        "/get-all-github-app-installed-repos",
                        web::get().to(get_all_github_app_installed_repos),
                    )
                    .route("/deploy-project", web::post().to(deploy_project_controller))
                    .route(
                        "/get-all-deployed-projects",
                        web::get().to(get_all_deployed_projects_controller),
                    )
                    .route(
                        "/get-project-detail",
                        web::get().to(get_project_details_controller),
                    ),
            )
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await?;

    Ok(())
}
