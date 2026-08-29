use std::fs;

use crate::{
    app::models::ProjectType,
    app_errors::AppError,
    core::{
        app_types::{DeployDetails, JobType, RedeployDetails, RunDetails, ShiprJson},
        config::app_config::get_worker_dir,
        infra::{detect::detect_project_type, process::run_script},
        worker::{
            api::host::Host,
            helpers::{build::build, install::install, pull::pull, run::run},
        },
    },
};

pub struct JobExecuter {
    host: Host,
}

impl Default for JobExecuter {
    fn default() -> Self {
        Self::new()
    }
}

impl JobExecuter {
    pub fn new() -> Self {
        Self { host: Host::new() }
    }

    fn get_env_export_cmd(&self, envs: &Option<Vec<crate::core::app_types::EnvVar>>) -> String {
        if let Some(env_vars) = envs {
            if env_vars.is_empty() {
                return "".to_string();
            }
            let exports = env_vars
                .iter()
                .map(|e| format!("export {}='{}'", e.key, e.value))
                .collect::<Vec<String>>()
                .join(" && ");
            format!("{} &&", exports)
        } else {
            "".to_string()
        }
    }

    fn get_project_path(&self, deploy_details: &DeployDetails) -> Result<String, AppError> {
        let repo_name = deploy_details.project_id.to_owned();

        if deploy_details.root_dir == "." {
            Ok(format!("/root/{}", repo_name))
        } else {
            Ok(format!("/root/{}/{}", repo_name, deploy_details.root_dir))
        }
    }

    pub async fn run(&self, run_details: &RunDetails) -> Result<(), AppError> {
        run(run_details).await?;

        Ok(())
    }

    pub async fn execute(
        &self,
        deploy_details: &DeployDetails,
        job_type: JobType,
        commit_hash: Option<String>,
    ) -> Result<(), AppError> {
        let steps_result = async {
            let (shipr_json_exists, commit_hash, branch) = if job_type == JobType::Deploy {
                pull(deploy_details, None).await?
            } else {
                pull(deploy_details, commit_hash).await?
            };

            let shipr_json =
                serde_json::from_str::<ShiprJson>(&fs::read_to_string("/root/shipr.json")?)?;

            install(deploy_details, &shipr_json).await?;
            build(deploy_details, &shipr_json).await?;

            let project_path = self.get_project_path(deploy_details)?;
            let project_type = detect_project_type(&project_path);

            Ok::<((String, Option<String>), ProjectType), AppError>((
                (commit_hash, branch),
                project_type,
            ))
        }
        .await;

        println!("steps result is: {:?}", steps_result);

        match steps_result {
            Ok(((commit_hash, branch), project_type)) => {
                println!("commit hash is: {:?}", commit_hash);
                println!("project type is: {:?}", project_type);
                self.host
                    .job_completed(
                        deploy_details.project_id.to_owned(),
                        job_type.clone(),
                        Some(commit_hash),
                        project_type,
                        branch,
                    )
                    .await?;

                // self.host
                //     .kill_vm(deploy_details.project_id.to_owned(), job_type)
                //     .await?;

                Ok(())
            }
            Err(e) => {
                println!("Job execution failed: {:?}", e);
                // let _ = self
                //     .host
                //     .kill_vm(deploy_details.project_id.to_owned(), job_type)
                //     .await;
                Err(e)
            }
        }
    }

    pub async fn redeploy(
        &self,
        redeploy_details: &RedeployDetails,
        job_type: JobType,
    ) -> Result<(), AppError> {
        println!("redeploy details is:");

        let project_id = redeploy_details.project_id.to_owned();

        // For dev
        let presigned_download_url = redeploy_details.presigned_download_url.to_owned().replace(
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/",
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/s3/",
        );

        println!("presigned download url is: {}", presigned_download_url);

        let download_cmd = format!("curl -o {}.zip '{}'", &project_id, presigned_download_url);

        println!("download command is: {}", download_cmd);

        run_script(vec![&download_cmd], get_worker_dir()).await?;

        println!("download command completed");

        let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

        println!("unzip command is: {}", unzip_cmd);

        run_script(vec![&unzip_cmd], get_worker_dir()).await?;

        println!("unzip command completed");

        let copy_job_json = format!("cp /root/{}/shipr/job.json /root/", project_id);

        run_script(vec![&copy_job_json], get_worker_dir()).await?;

        println!("copy job json completed");

        let job_json_str = fs::read_to_string("/root/job.json")?;

        let mut job_json = serde_json::from_str::<DeployDetails>(&job_json_str)?;

        println!("job json is: ");

        let presigned_upload_url = redeploy_details.presigned_upload_url.to_owned();

        println!("presigned upload url is: {}", presigned_upload_url);

        job_json.presigned_upload_url = presigned_upload_url;
        job_json.installation_access_token = redeploy_details.access_token.to_owned();
        job_json.envs = redeploy_details.envs.clone();

        if let Some(branch) = &redeploy_details.branch {
            job_json.branch = Some(branch.clone());
        }

        println!("reached here");

        let rm_previous_project = format!("rm -rf /root/{}*", project_id);

        run_script(vec![&rm_previous_project], get_worker_dir()).await?;

        let commit_hash = redeploy_details.commit_hash.clone();

        fs::write("/root/job.json", serde_json::to_string(&job_json)?)?;

        let rm_previous_project = format!("rm -rf /root/{}*", project_id);

        run_script(vec![&rm_previous_project], get_worker_dir()).await?;

        self.execute(&job_json, job_type, Some(commit_hash)).await?;

        let project_path = self.get_project_path(&job_json)?;
        let project_type = detect_project_type(&project_path);

        self.host
            .job_completed(
                redeploy_details.project_id.clone(),
                JobType::Redeploy,
                Some(redeploy_details.commit_hash.clone()),
                project_type,
                redeploy_details.branch.clone(),
            )
            .await?;

        Ok(())
    }
}
