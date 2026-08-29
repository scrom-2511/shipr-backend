use std::time::Duration;

use actix_web::web;
use futures::StreamExt;
use lapin::options::BasicAckOptions;

use crate::{
    app_errors::AppError,
    core::{
        app_types::{DeployDetails, DeployReq},
        controller::{
            dispatcher::job_dispatcher::JobDispatcher,
            queue::deploy_queue::DeployQueue,
            storage::s3::S3Service,
            vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
        },
    },
    shared::github_app::GithubApp,
};

pub async fn listen_deploy(
    s3_service: S3Service,
    mut job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    deploy_queue: web::Data<DeployQueue>,
) -> Result<(), AppError> {
    let mut consumer = deploy_queue
        .create_consumer("deploy-worker")
        .await
        .map_err(|e| AppError::LapinError(e.to_string()))?;

    loop {
        let delivery = match consumer.next().await {
            Some(Ok(delivery)) => delivery,
            Some(Err(e)) => {
                eprintln!("RabbitMQ error: {:?}", e);
                continue;
            }
            None => return Ok(()),
        };

        let deploy_details_req = serde_json::from_slice::<DeployReq>(&delivery.data)?;

        let installation_id = deploy_details_req.installation_id;

        println!(
            "[DEPLOY] {} - fetching GitHub access token",
            deploy_details_req.project_id
        );

        let github = GithubApp::new();

        println!(
            "[DEPLOY] {} - BEFORE get_installation_access_token",
            deploy_details_req.project_id
        );

        let access_token = github
            .get_installation_access_token(installation_id)
            .await
            .unwrap();

        println!(
            "[DEPLOY] {} - AFTER get_installation_access_token",
            deploy_details_req.project_id
        );

        let url = format!("https://github.com/{}.git", deploy_details_req.full_name);

        let cleaned_url = url.replace(".git", "");

        println!("Cleaned URL: {}", cleaned_url);

        let project_id = deploy_details_req.project_id;

        let presigned_upload_url = s3_service
            .get_presigned_upload_url(&project_id)
            .await
            .unwrap();

        println!("Access Token fetched");

        let deploy_details = DeployDetails {
            branch: Some(deploy_details_req.branch),
            full_name: deploy_details_req.full_name,
            presigned_upload_url,
            root_dir: deploy_details_req.root_dir,
            installation_access_token: access_token,
            envs: Some(deploy_details_req.envs),
            project_id,
        };

        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        if let Err(e) = job_dispatcher.dispatch_deploy_job(&deploy_details).await {
            eprintln!("Job dispatch failed: {:?}", e);
        }

        delivery
            .ack(BasicAckOptions::default())
            .await
            .map_err(|e| AppError::LapinError(e.to_string()))?;

        tokio::task::spawn(async move {
            let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
            if let Err(e) = new_vm.create_hot_vm(&vm_pool).await {
                eprintln!("Failed to create VM: {:?}", e);
            }
        });
    }
}
