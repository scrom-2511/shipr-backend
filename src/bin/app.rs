use std::error::Error;

use actix_web::middleware::from_fn;
use actix_web::{App, HttpServer, web};
use dodopayments::Client;
use dotenvy::dotenv;
use sea_orm::ConnectOptions;
use shipr::app::controllers::auth::github_signup::{github_auth_url, github_callback};
use shipr::app::controllers::auth::signin::signin_controller;
use shipr::app::controllers::auth::signup::signup_controller;
use shipr::app::controllers::billing::dodo_checkout::dodo_checkout_controller;
use shipr::app::controllers::billing::dodo_webhook::dodo_webhook_controller;
use shipr::app::controllers::billing::get_billing_details::get_billing_details_controller;
use shipr::app::controllers::billing::onDemand::auto_top_up;
use shipr::app::controllers::billing::onDemand::dodo_ondemand_checkout::dodo_ondemand_checkout_controller;
use shipr::app::controllers::billing::payment_confirmation::payment_confirmation_controller;
// use shipr::app::controllers::billing::topup::topup_handler;
use shipr::app::controllers::github::get_state::get_state;
use shipr::app::controllers::github::update_userid_github_app_installations::update_userid_github_app_installations;
use shipr::app::controllers::project::add_new_project::add_new_project;
use shipr::app::controllers::project::check_name_availability::check_repo_name_availability;
use shipr::app::controllers::project::delete_project::delete_project_controller;
use shipr::app::controllers::project::deploy_project::deploy_project_controller;
use shipr::app::controllers::project::get_all_deployed_projects::get_all_deployed_projects_controller;
use shipr::app::controllers::project::get_all_github_app_installed_repos::get_all_github_app_installed_repos;
use shipr::app::controllers::project::get_project_details::get_project_details_controller;
use shipr::app::controllers::project::get_project_traffic::get_project_traffic_controller;
use shipr::app::controllers::project::job_completed::job_completed_controller;
use shipr::app::state::AppState;

use shipr::app::controllers::project::kill_vm_controller::kill_vm_controller;
use shipr::app::middlewares::is_logged_in::is_logged_in;
use shipr::app::webhooks::github_event::github_event;
use shipr::core::controller::cli::listen_deploy::listen_deploy;
use shipr::core::controller::cli::listen_idle_kill::listen_idle_kill;
use shipr::core::controller::cli::listen_redeploy::listen_redeploy;
use shipr::core::controller::dispatcher::job_dispatcher::JobDispatcher;
use shipr::core::controller::queue::deploy_queue::DeployQueue;
use shipr::core::controller::queue::idle_kill_queue::IdleKillQueue;
use shipr::core::controller::queue::lapin::Lapin;
use shipr::core::controller::queue::redeploy_queue::ReDeployQueue;
use shipr::core::controller::storage::redis::Redis;
use shipr::core::controller::storage::s3::S3Service;
use shipr::core::controller::vm::heartbeat_store::HeartbeatStore;
use shipr::core::controller::vm::id_allocator::IdAllocator;
use shipr::core::controller::vm::vm_pool::VmPool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let db_url = std::env::var("DATABASE_URL");

    if db_url.is_err() {
        return Err("DATABASE_URL not found".into());
    }

    let pool = sea_orm::Database::connect(&db_url.unwrap()).await?;

    println!("Connected to database");

    let product_id =
        std::env::var("DODO_PRODUCT_ID").unwrap_or_else(|_| "pdt_compute_credits".to_string());

    let client = Client::from_env();

    let client = match client {
        Ok(c) => {
            println!("{:?}", c);
            c
        }
        Err(_) => return Err("Failed to load DODO credentials".into()),
    };

    let client_arc = std::sync::Arc::new(client);

    let app_state = web::Data::new(AppState {
        db: pool.clone(),
        client: client_arc.clone(),
        product_id,
    });

    println!("Server running on port 9000");

    let redis = Redis::new().await;
    let id_allocator = IdAllocator::new(redis.clone());
    let vm_pool = VmPool::new(redis.clone(), id_allocator.clone());
    let s3_service = S3Service::new().await;
    let heartbeat_store = HeartbeatStore::new(redis.clone());

    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
        pool.clone(),
    );

    // for _ in 0..1 {
    //     let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
    //     new_vm.create_hot_vm(&vm_pool).await?;
    // }

    let lapin_conn = Lapin::new().await?;
    let deploy_queue = web::Data::new(DeployQueue::new(&lapin_conn).await?);
    let redeploy_queue = web::Data::new(ReDeployQueue::new(&lapin_conn).await?);
    let idle_kill_queue = web::Data::new(IdleKillQueue::new(&lapin_conn).await?);

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
        let pool_data = web::Data::new(pool.clone());

        tokio::spawn(async move {
            listen_idle_kill(
                idle_kill_queue,
                vm_pool,
                id_allocator,
                pool_data,
                redis.clone(),
            )
            .await;
        });
    }

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allowed_origin("https://healthcare-camcorders-oecd-flour.trycloudflare.com")
            .allowed_origin("http://localhost:5173")
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(deploy_queue.clone())
            .app_data(redeploy_queue.clone())
            .app_data(idle_kill_queue.clone())
            .app_data(web::Data::new(id_allocator.clone()))
            .app_data(web::Data::new(vm_pool.clone()))
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            // .route("/api/billing/topup", web::post().to(topup_handler))
            .route("/webhooks/dodo", web::post().to(dodo_webhook_controller))
            .route("/signup", web::post().to(signup_controller))
            .route("/signin", web::post().to(signin_controller))
            .route("/add-project", web::post().to(add_new_project))
            .route("/auth/github", web::get().to(github_auth_url))
            .route("/auth/github/callback", web::get().to(github_callback))
            .route("/webhook/github", web::post().to(github_event))
            .route(
                "/check-repo-name-availability",
                web::post().to(check_repo_name_availability),
            )
            .route("/job-completed", web::post().to(job_completed_controller))
            .route("/kill-vm", web::post().to(kill_vm_controller))
            .route(
                "/on-demand-checkout",
                web::post().to(dodo_ondemand_checkout_controller),
            )
            .route("/auto-top-up", web::post().to(auto_top_up::auto_top_up))
            .route(
                "/webhook/dodo-payments",
                web::post().to(dodo_webhook_controller),
            )
            // .route(
            //     "/api/webhooks/dodo-payments",
            //     web::post().to(dodo_webhook_controller),
            // )
            // .route("/api/checkout", web::post().to(dodo_checkout_controller))
            .service(
                web::scope("")
                    .wrap(from_fn(is_logged_in))
                    .route("/get-state", web::get().to(get_state))
                    .route("/checkout", web::post().to(dodo_checkout_controller))
                    .route(
                        "/payment-confirmation",
                        web::get().to(payment_confirmation_controller),
                    )
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
                    )
                    .route(
                        "/get-project-traffic",
                        web::get().to(get_project_traffic_controller),
                    )
                    .route(
                        "/delete-project",
                        web::delete().to(delete_project_controller),
                    )
                    .route(
                        "/get-billing-details",
                        web::get().to(get_billing_details_controller),
                    )
                    .route("/api/checkout", web::post().to(dodo_checkout_controller))
                    .route("/checkout", web::post().to(dodo_checkout_controller)),
            )
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await?;

    Ok(())
}
