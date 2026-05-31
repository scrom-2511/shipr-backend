use core::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

use crate::app::db::DbPool;
use crate::app_errors::AppError;
use crate::core::app_types::{DeployDetails, EnvVar, JobType, RedeployDetails, RunDetails};
use crate::core::config::app_config::get_dir;
use crate::core::controller::storage::s3::S3Service;
use crate::core::controller::vm::firecracker::Firecracker;
use crate::core::controller::vm::heartbeat_store::HeartbeatStore;
use crate::core::controller::vm::id_allocator::IdAllocator;
use crate::core::controller::vm::vm_pool::VmPool;
use crate::core::infra::kill_vm::kill_vm;
use crate::core::infra::process::run_script;
use std::collections::HashMap;

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Deploy => write!(f, "deploy"),
            JobType::Run => write!(f, "run"),
            JobType::Redeploy => write!(f, "redeploy"),
        }
    }
}

pub trait VmDetails {
    fn get_json(&self) -> String;
    fn get_project_id(&self) -> String;
    fn get_job_type(&self) -> JobType;
}

impl VmDetails for DeployDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Deploy
    }
}

impl VmDetails for RunDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Run
    }
}

impl VmDetails for RedeployDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Redeploy
    }
}

#[derive(Clone)]
pub struct JobDispatcher {
    vm_pool: VmPool,
    pub s3_service: S3Service,
    id_allocator: IdAllocator,
    heartbeat_store: HeartbeatStore,
    pool: DbPool,
}

impl JobDispatcher {
    pub fn new(
        vm_pool: VmPool,
        s3_service: S3Service,
        id_allocator: IdAllocator,
        heartbeat_store: HeartbeatStore,
        pool: DbPool,
    ) -> Self {
        Self {
            vm_pool,
            s3_service,
            id_allocator,
            heartbeat_store,
            pool,
        }
    }

    fn git_url_validator(&self, git_url: &str) -> Result<bool, AppError> {
        if let Ok(url) = Url::parse(git_url)
            && url.host_str() == Some("github.com")
            && url.scheme() == "https"
        {
            return Ok(true);
        }

        Err(AppError::InvalidGitUrl)
    }

    async fn move_json_to_vm(
        &self,
        vm: &Firecracker,
        vm_details: &impl VmDetails,
    ) -> Result<(), AppError> {
        vm.get_base_id();

        println!("vm base_id is: {}", vm.get_base_id());

        let job_json = vm_details.get_json();

        let project_id = vm_details.get_project_id();

        let job_type = vm_details.get_job_type().to_string().to_lowercase();

        let job_json_path = format!(
            "/home/scrom/code/shipr/job_jsons/{}/{}.json",
            job_type, project_id
        );

        if let Some(parent) = PathBuf::from(&job_json_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&job_json_path, job_json)?;

        let copy_job_json_to_vm = format!(
            "scp -r -i ubuntu.id_rsa {} root@172.16.0.{}:/root/job.json",
            job_json_path,
            vm.get_base_id() + 2
        );

        run_script(vec![&copy_job_json_to_vm], get_dir()).await?;

        Ok(())
    }

    pub async fn dispatch_deploy_job(
        &mut self,
        deploy_details: &DeployDetails,
    ) -> Result<(), AppError> {
        let vm = self
            .vm_pool
            .get_or_create_vm(&deploy_details.project_id, &JobType::Deploy)
            .await?
            .0;

        println!("vm is xyz");

        let base_id = vm.get_base_id();
        println!("vm id is: {}", base_id);

        self.move_json_to_vm(&vm, deploy_details).await?;

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        println!("vm ip is: {}", vm_ip);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
            get_dir(),
        ).await?;

        println!("scp cmd run completed");

        vm.execute_command("cd /root && ./worker job.json deploy")
            .await?;

        println!("execute command completed");

        Ok(())
    }

    pub async fn dispatch_redeploy_job(
        &mut self,
        redeploy_details: &mut RedeployDetails,
    ) -> Result<(), AppError> {
        let (vm, _) = self
            .vm_pool
            .get_or_create_vm(&redeploy_details.project_id, &JobType::Redeploy)
            .await?;

        let row: (Option<Vec<String>>,) =
            sqlx::query_as("SELECT envs FROM projects WHERE project_id = $1")
                .bind(&redeploy_details.project_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let envs: Option<Vec<EnvVar>> = match row.0 {
            Some(encrypted_envs_vec) => {
                if let Some(encrypted_envs) = encrypted_envs_vec.first() {
                    let decrypted = crate::shared::crypto::Crypto::decrypt(encrypted_envs);
                    Some(serde_json::from_str(&decrypted).map_err(|e| {
                        AppError::Database(format!("Failed to deserialize envs: {}", e))
                    })?)
                } else {
                    None
                }
            }
            None => None,
        };

        redeploy_details.envs = envs;

        self.move_json_to_vm(&vm, redeploy_details).await?;

        let base_id = vm.get_base_id();

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
            get_dir(),
        ).await?;

        vm.execute_command("cd /root && ./worker job.json redeploy")
            .await?;

        Ok(())
    }

    pub async fn dispatch_run_job(&mut self, project_id: &str) -> Result<(), AppError> {
        let (vm, is_new) = self
            .vm_pool
            .get_or_create_vm(project_id, &JobType::Run)
            .await?;

        println!("is new is: {}", is_new);

        let base_id = vm.get_base_id();

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
            get_dir(),
        ).await?;

        println!("cp cmd run completed");

        if is_new {
            let presigned_download_url = self
                .s3_service
                .get_presigned_download_url(project_id)
                .await?;

            let row: (Option<Vec<String>>, Option<Vec<String>>) =
                sqlx::query_as("SELECT envs, run_cmds FROM projects WHERE project_id = $1")
                    .bind(project_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            let envs: Option<Vec<EnvVar>> = match row.0 {
                Some(encrypted_envs_vec) => {
                    if let Some(encrypted_envs) = encrypted_envs_vec.first() {
                        let decrypted = crate::shared::crypto::Crypto::decrypt(encrypted_envs);
                        Some(serde_json::from_str(&decrypted).map_err(|e| {
                            AppError::Database(format!("Failed to deserialize envs: {}", e))
                        })?)
                    } else {
                        None
                    }
                }
                None => None,
            };

            let run_command = row.1.map(|cmds| cmds.join(" && ")).unwrap_or_default();

            let run_details = RunDetails {
                presigned_download_url,
                run_command,
                project_id: project_id.to_string(),
                envs,
            };

            self.move_json_to_vm(&vm, &run_details).await?;

            let id_allocator = self.id_allocator.clone();
            let vm_pool = self.vm_pool.clone();
            let heartbeat_store = self.heartbeat_store.clone();
            let project_id = project_id.to_string();

            vm.execute_command_bg("cd /root && ./worker job.json run")?;

            tokio::task::spawn(async move {
                let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;

                new_vm.create_hot_vm(&vm_pool).await?;

                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    let dead = heartbeat_store.is_dead(&project_id).await?;

                    if dead {
                        // kill_vm(&project_id, &JobType::Run, &vm_pool, &id_allocator).await?;
                        break;
                    }
                }

                Ok::<(), AppError>(())
            });
        }

        Ok(())
    }
}
