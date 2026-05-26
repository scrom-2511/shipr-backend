use std::{fs, net::UdpSocket, thread::sleep, time::Duration};

use tokio::net::TcpStream;

use crate::{
    app_errors::AppError,
    core::{
        app_types::{DeployDetails, JobType, ProjectType, RedeployDetails, RunDetails},
        config::{app_config::get_worker_dir, project_default_config::get_default_config},
        infra::{
            detect::detect_project_type,
            process::{run_script, run_script_bg},
        },
        worker::api::host::Host,
    },
    shared::github_app::GithubApp,
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

    fn extract_repo_name(&self, url: &str) -> Result<String, AppError> {
        let url = url.trim();

        let last_part = url.split('/').next_back().ok_or(AppError::InvalidGitUrl)?;

        let repo_name = last_part.strip_suffix(".git").unwrap_or(last_part);

        Ok(repo_name.to_string())
    }

    fn get_project_path(&self, deploy_details: &DeployDetails) -> Result<String, AppError> {
        let repo_name = deploy_details.project_id.to_owned();

        if deploy_details.home_dir.is_empty() {
            Ok(format!("/root/{}", repo_name))
        } else {
            Ok(format!("/root/{}/{}", repo_name, deploy_details.home_dir))
        }
    }

    async fn pull(
        &self,
        deploy_details: &DeployDetails,
        commit_hash: Option<String>,
    ) -> Result<String, AppError> {
        let github_app = GithubApp::new();

        // self.host
        //     .send_logs(&deploy_details.project_id, "Pulling repository...")
        //     .await?;

        let (owner, repo) = {
            let parts: Vec<&str> = deploy_details.full_name.split('/').collect();
            (parts[0].to_string(), parts[1].to_string())
        };

        println!("owner is: {}", owner);
        println!("repo is: {}", repo);

        let commit_hash = if commit_hash.is_none() {
            github_app
                .get_commit_sha(
                    deploy_details.branch.clone(),
                    &owner,
                    &repo,
                    &deploy_details.installation_access_token,
                )
                .await?
        } else {
            commit_hash.unwrap()
        };

        let tarball_url = github_app
            .get_tarball_url_from_commit_hash(&commit_hash, &owner, &repo)
            .await?;
        println!("tarball url is: {}", tarball_url);

        let git_pull_cmd = format!(
            "curl -Lo {}.tar.gz {} -H 'Accept: application/vnd.github.v3+json' -H 'Authorization: token {}'",
            deploy_details.project_id, tarball_url, deploy_details.installation_access_token
        );

        println!("git clone command is: {}", git_pull_cmd);

        let extract_cmd = format!("tar -xzf {}.tar.gz", deploy_details.project_id);

        self.host
            .send_logs(&deploy_details.project_id, "Extracting repository...")
            .await?;

        println!("extract command is: {}", extract_cmd);

        let rename_cmd = format!(
            "mv {}-* {}",
            deploy_details.full_name.replace("/", "-"),
            deploy_details.project_id
        );

        println!("rename command is: {}", rename_cmd);

        run_script(
            vec![&git_pull_cmd, &extract_cmd, &rename_cmd],
            get_worker_dir(),
        )?;

        Ok(commit_hash)
    }

    async fn install(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;

        println!("project path is: {}", project_path);

        self.host
            .send_logs(&deploy_details.project_id, "Installing dependencies...")
            .await?;

        if deploy_details.install_commands.is_some() {
            self.host
                .send_logs(
                    &deploy_details.project_id,
                    "Found custom install commands, using them...",
                )
                .await?;
            let install_cmd = deploy_details
                .install_commands
                .as_ref()
                .unwrap()
                .join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, install_cmd);

            run_script(vec![&final_cmd], get_worker_dir())?;

            return Ok(());
        }

        self.host
            .send_logs(
                &deploy_details.project_id,
                "No custom install commands found, using default install commands...",
            )
            .await?;

        let project_type = detect_project_type(&project_path);

        self.host
            .send_logs(
                &deploy_details.project_id,
                &format!("Detected project type: {}", project_type),
            )
            .await?;

        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.install_commands.join(" && ")
        );

        println!("final command is: {}", final_cmd);

        run_script(vec![&final_cmd], get_worker_dir())?;

        self.host
            .send_logs(
                &deploy_details.project_id,
                "Dependencies installed successfully",
            )
            .await?;

        Ok(())
    }

    async fn build(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;

        self.host
            .send_logs(&deploy_details.project_id, "Building project...")
            .await?;

        if deploy_details.build_commands.is_some() {
            self.host
                .send_logs(
                    &deploy_details.project_id,
                    "Found custom build commands, using them...",
                )
                .await?;
            let build_cmd = deploy_details.build_commands.as_ref().unwrap().join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, build_cmd);

            run_script(vec![&final_cmd], get_worker_dir())?;
            return Ok(());
        }

        self.host
            .send_logs(
                &deploy_details.project_id,
                "No custom build commands found, using default build commands...",
            )
            .await?;

        let project_type = detect_project_type(&project_path);
        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.build_commands.join(" && ")
        );

        println!("final command is: {}", final_cmd);

        run_script(vec![&final_cmd], get_worker_dir())?;

        let zip_cmd = format!(
            "cd /root/{}/{} && zip -r /root/{}.zip . /root/job.json",
            deploy_details.project_id, deploy_details.dist_dir, deploy_details.project_id
        );

        println!("zip command is: {}", zip_cmd);

        run_script(vec![&zip_cmd], get_worker_dir())?;

        // For dev
        let presigned_upload_url = deploy_details.presigned_upload_url.to_owned().replace(
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/",
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/s3/",
        );

        let upload_cmd = format!(
            "curl -X PUT -T {}.zip '{}'",
            deploy_details.project_id, presigned_upload_url
        );

        run_script(vec![&upload_cmd], get_worker_dir())?;

        self.host
            .send_logs(&deploy_details.project_id, "Build completed successfully")
            .await?;

        Ok(())
    }

    pub async fn execute(
        &self,
        deploy_details: &DeployDetails,
        job_type: JobType,
        commit_hash: Option<String>,
    ) -> Result<(), AppError> {
        let commit_hash = if job_type == JobType::Deploy {
            self.pull(deploy_details, None).await?
        } else {
            self.pull(deploy_details, commit_hash).await?
        };

        self.install(deploy_details).await?;

        self.build(deploy_details).await?;

        let host = Host::new();

        let project_path = self.get_project_path(deploy_details)?;
        let project_type = detect_project_type(&project_path);

        host.job_completed(
            deploy_details.full_name.to_owned(),
            job_type.clone(),
            Some(commit_hash),
            project_type,
        )
        .await?;

        host.kill_vm(deploy_details.project_id.to_owned(), job_type)
            .await?;

        Ok(())
    }

    pub async fn run(&self, run_details: &RunDetails) -> Result<(), AppError> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;

        socket.connect("8.8.8.8:80")?;

        let local_ip = socket.local_addr()?.ip();

        let port_exists = TcpStream::connect(format!("{}:3000", local_ip))
            .await
            .is_ok();

        println!("port exists is: {}", port_exists);

        if port_exists {
            return Ok(());
        }

        let project_id = &run_details.project_id;

        // For dev
        let presigned_download_url = run_details.presigned_download_url.to_owned().replace(
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/",
            "https://francisco-unscholarlike-punctually.ngrok-free.dev/s3/",
        );

        let download_cmd = format!("curl -o {}.zip '{}'", project_id, presigned_download_url);

        run_script(vec![&download_cmd], get_worker_dir())?;

        let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

        run_script(vec![&unzip_cmd], get_worker_dir())?;

        sleep(Duration::from_secs(3));

        let project_path = format!("/root/{}", project_id);

        let project_type = detect_project_type(&project_path);

        if !run_details.run_command.is_empty() {
            let run_cmd = format!("cd {} && {}", project_path, run_details.run_command);

            run_script_bg(vec![&run_cmd], get_worker_dir())?;

            return Ok(());
        }

        let config = get_default_config(project_type);
        let config_run_cmd = config.run_command.unwrap();

        let final_cmd = format!("cd {} && {}", project_path, config_run_cmd);

        run_script_bg(vec![&final_cmd], get_worker_dir())?;

        println!("Run command completed");
        Ok(())
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

        run_script(vec![&download_cmd], get_worker_dir())?;

        println!("download command completed");

        let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

        println!("unzip command is: {}", unzip_cmd);

        run_script(vec![&unzip_cmd], get_worker_dir())?;

        println!("unzip command completed");

        let copy_job_json = format!("cp /root/{}/job.json /root/", project_id);

        run_script(vec![&copy_job_json], get_worker_dir())?;

        println!("copy job json completed");

        let job_json_str = fs::read_to_string(format!("/root/{}/root/job.json", project_id))?;

        let mut job_json = serde_json::from_str::<DeployDetails>(&job_json_str)?;

        println!("job json is: ");

        // For dev
        let presigned_upload_url = redeploy_details.presigned_upload_url.to_owned();

        println!("presigned upload url is: {}", presigned_upload_url);

        job_json.presigned_upload_url = presigned_upload_url;
        job_json.installation_access_token = redeploy_details.access_token.to_owned();

        if let Some(branch) = &redeploy_details.branch {
            job_json.branch = Some(branch.clone());
        }

        println!("reached here");

        let rm_previous_project = format!("rm -rf /root/{}*", project_id);

        run_script(vec![&rm_previous_project], get_worker_dir())?;

        let commit_hash = redeploy_details.commit_hash.clone();

        self.execute(&job_json, job_type, Some(commit_hash)).await?;

        Ok(())
    }
}
