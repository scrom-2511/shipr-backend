use std::time::Duration;

use actix_web::web;

use crate::{
    app::db::DbPool,
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
    job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    redeploy_queue: web::Data<ReDeployQueue>,
    pool: web::Data<DbPool>,
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

        let incoming_branch = redeploy_event.ref_field.replace("refs/heads/", "");
        let full_name = &redeploy_event.repository.full_name;

        let mut changed_files = std::collections::HashSet::new();
        for commit in &redeploy_event.commits {
            for file in &commit.added {
                changed_files.insert(file.clone());
            }
            for file in &commit.modified {
                changed_files.insert(file.clone());
            }
            for file in &commit.removed {
                changed_files.insert(file.clone());
            }
        }

        let projects: Vec<crate::app::models::Project> =
            match sqlx::query_as("SELECT * FROM projects WHERE full_name = $1 AND branch = $2")
                .bind(full_name)
                .bind(&incoming_branch)
                .fetch_all(pool.as_ref())
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    continue;
                }
            };

        for project in projects {
            let root_dir = project.root_dir.trim_matches('/');
            let should_redeploy = if root_dir.is_empty() || root_dir == "." {
                true
            } else {
                changed_files
                    .iter()
                    .any(|file| file == root_dir || file.starts_with(&format!("{}/", root_dir)))
            };

            if !should_redeploy {
                println!(
                    "Skipping redeploy for project {} (root_dir: {}) - no relevant changes.",
                    project.project_id, project.root_dir
                );
                continue;
            }

            // println!("Triggering redeploy for project: {}", project.project_id);

            // let project_id = &project.project_id;
            // let github = GithubApp::new();

            // let presigned_upload_url = match s3_service.get_presigned_upload_url(project_id).await {
            //     Ok(url) => url,
            //     Err(e) => {
            //         eprintln!("Failed to get presigned upload url: {:?}", e);
            //         continue;
            //     }
            // };

            // let presigned_download_url =
            //     match s3_service.get_presigned_download_url(project_id).await {
            //         Ok(url) => url,
            //         Err(e) => {
            //             eprintln!("Failed to get presigned download url: {:?}", e);
            //             continue;
            //         }
            //     };

            // let access_token = match github
            //     .get_installation_access_token(redeploy_event.installation.id)
            //     .await
            // {
            //     Ok(token) => token,
            //     Err(e) => {
            //         eprintln!("Failed to get installation access token: {:?}", e);
            //         continue;
            //     }
            // };

            println!("Triggering redeploy for project: {}", project.project_id);

            let project_id = &project.project_id;
            let github = GithubApp::new();

            // Run all three independent async calls concurrently
            let (presigned_upload_url, presigned_download_url, access_token) = match tokio::try_join!(
                s3_service.get_presigned_upload_url(project_id),
                s3_service.get_presigned_download_url(project_id),
                github.get_installation_access_token(redeploy_event.installation.id),
            ) {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("Failed during concurrent setup: {:?}", e);
                    continue;
                }
            };

            let envs = if let Some(envs_vec) = &project.envs {
                if let Some(encrypted_envs) = envs_vec.first() {
                    let json = crate::shared::crypto::Crypto::decrypt(encrypted_envs);
                    serde_json::from_str::<Vec<crate::core::app_types::EnvVar>>(&json).ok()
                } else {
                    None
                }
            } else {
                None
            };

            let mut redeploy_details = RedeployDetails {
                commit_hash: redeploy_event.after.clone(),
                presigned_download_url,
                presigned_upload_url,
                project_id: project_id.to_owned(),
                access_token,
                branch: Some(incoming_branch.clone()),
                envs,
            };

            // let id_allocator = id_allocator.clone();
            // let vm_pool = vm_pool.clone();

            // tokio::task::spawn(async move {
            //     let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
            //     if let Err(e) = new_vm.create_hot_vm(&vm_pool).await {
            //         eprintln!("Failed to create VM: {:?}", e);
            //     }
            // });

            // if let Err(e) = job_dispatcher
            //     .dispatch_redeploy_job(&mut redeploy_details)
            //     .await
            // {
            //     eprintln!("Job dispatch failed: {:?}", e);
            // }

            if let Err(e) = job_dispatcher
                .dispatch_redeploy_job(&mut redeploy_details)
                .await
            {
                eprintln!("Job dispatch failed: {:?}", e);
                continue;
            }

            let id_allocator = id_allocator.clone();
            let vm_pool = vm_pool.clone();
            tokio::task::spawn(async move {
                let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
                if let Err(e) = new_vm.create_hot_vm(&vm_pool).await {
                    eprintln!("Failed to create VM: {:?}", e);
                }
            });
        }
    }
}
