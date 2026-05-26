use std::time::Duration;

use actix_web::web;

use crate::{
    app_errors::AppError,
    core::{
        app_types::RedeployDetails,
        controller::{
            dispatcher::job_dispatcher::JobDispatcher,
            queue::redeploy_queue::ReDeployQueue,
            storage::s3::S3Service,
            vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
        },
    },
    shared::github_app::GithubApp,
};

pub async fn listen_redeploy(
    s3_service: S3Service,
    mut job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    redeploy_queue: web::Data<ReDeployQueue>,
) {
    loop {
        let redeploy_event = match redeploy_queue.consume().await {
            Ok(ev) => ev,
            Err(e) => {
                eprintln!("Queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let project_id = redeploy_event.repository.full_name.replace("/", "~");

        let (_owner, _repo) = {
            let parts: Vec<&str> = redeploy_event.repository.full_name.split('/').collect();
            (parts[0].to_string(), parts[1].to_string())
        };

        let github = GithubApp::new();

        let presigned_upload_url = match s3_service.get_presigned_upload_url(&project_id).await {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Failed to get presigned upload url: {:?}", e);
                continue;
            }
        };

        let presigned_download_url = match s3_service.get_presigned_download_url(&project_id).await
        {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Failed to get presigned download url: {:?}", e);
                continue;
            }
        };

        let access_token = match github
            .get_installation_access_token(redeploy_event.installation.id)
            .await
        {
            Ok(token) => token,
            Err(e) => {
                eprintln!("Failed to get installation access token: {:?}", e);
                continue;
            }
        };

        let branch = redeploy_event
            .ref_field
            .split('/')
            .last()
            .map(|s| s.to_string());

        let redeploy_details = RedeployDetails {
            commit_hash: redeploy_event.after,
            presigned_download_url,
            presigned_upload_url,
            project_id: project_id.to_owned(),
            access_token,
            branch,
        };

        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        tokio::task::spawn(async move {
            let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
            new_vm.create_new_vm_and_add_to_pool(&vm_pool).await?;

            Ok::<(), AppError>(())
        });

        if let Err(e) = job_dispatcher
            .dispatch_redeploy_job(&redeploy_details)
            .await
        {
            eprintln!("Job dispatch failed: {:?}", e);
        }
    }
}
