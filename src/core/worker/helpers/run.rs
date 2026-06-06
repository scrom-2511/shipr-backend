use std::{net::UdpSocket, thread::sleep, time::Duration};

use tokio::net::TcpStream;

use crate::{
    app_errors::AppError,
    core::{
        app_types::{DeployDetails, RunDetails},
        config::{app_config::get_worker_dir, project_default_config::get_default_config},
        infra::{
            detect::detect_project_type,
            process::{run_script, run_script_bg},
        },
        worker::helpers::install::get_install_cmds,
    },
};

pub async fn run(run_details: &RunDetails) -> Result<(), AppError> {
    let port_exists = check_port_exists().await?;

    if port_exists {
        return Ok(());
    }

    sleep(Duration::from_secs(3));

    download_project(run_details).await?;

    unzip_project(&run_details.project_id).await?;

    run_cmds(run_details).await?;

    println!("Run command completed");
    Ok(())
}

fn get_env_export_cmd(envs: &Option<Vec<crate::core::app_types::EnvVar>>) -> String {
    if let Some(env_vars) = envs {
        if env_vars.is_empty() {
            return "".to_string();
        }
        let exports = env_vars
            .iter()
            .map(|e| format!("export {}='{}'", e.key, e.value))
            .collect::<Vec<String>>()
            .join(" && ");
        exports.to_string()
    } else {
        "".to_string()
    }
}

async fn check_port_exists() -> Result<bool, AppError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    socket.connect("8.8.8.8:80")?;

    let local_ip = socket.local_addr()?.ip();

    let port_exists = TcpStream::connect(format!("{}:3000", local_ip))
        .await
        .is_ok();

    println!("port exists is: {}", port_exists);

    Ok(port_exists)
}

async fn download_project(run_details: &RunDetails) -> Result<(), AppError> {
    let project_id = &run_details.project_id;

    let presigned_download_url = run_details.presigned_download_url.to_owned().replace(
        "https://francisco-unscholarlike-punctually.ngrok-free.dev/",
        "https://francisco-unscholarlike-punctually.ngrok-free.dev/s3/",
    );

    let download_cmd = format!("curl -o {}.zip '{}'", project_id, presigned_download_url);

    run_script(vec![&download_cmd], get_worker_dir()).await?;

    Ok(())
}

async fn unzip_project(project_id: &str) -> Result<(), AppError> {
    let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

    run_script(vec![&unzip_cmd], get_worker_dir()).await?;

    Ok(())
}

async fn get_run_cmds(run_details: &RunDetails) -> Result<String, AppError> {
    let project_path = format!("/root/{}", run_details.project_id);

    if !run_details.run_command.is_empty() {
        let run_cmds = run_details.run_command.join(" && ");

        return Ok(run_cmds);
    }

    let project_type = detect_project_type(&project_path);

    let default_config = get_default_config(&project_type);
    let config_run_cmd = default_config.run_command.unwrap().join("&&");

    Ok(config_run_cmd)
}

async fn get_project_path_to_run(run_details: &RunDetails) -> String {
    let base_path = format!("/root/{}", run_details.project_id);

    if !run_details.dist_dir.is_empty() {
        return format!("{}/{}", base_path, run_details.dist_dir);
    }

    let project_type = detect_project_type(&base_path);

    let default_config = get_default_config(&project_type);

    format!("{}/{}", base_path, default_config.dist_dir)
}

async fn run_cmds(run_details: &RunDetails) -> Result<(), AppError> {
    let run_cmd = get_run_cmds(run_details).await?;

    let env_cmd = get_env_export_cmd(&run_details.envs);

    let deploy_details =
        std::fs::read_to_string(format!("/root/{}/shipr/job.json", run_details.project_id))?;

    let deploy_details: DeployDetails = serde_json::from_str(&deploy_details)?;

    let install_cmd = get_install_cmds(&deploy_details)?;

    let install_cmd_path = format!("/root/{}", run_details.project_id);

    let install_cmd = format!(
        "cd {} && {} 2>/dev/null || true",
        install_cmd_path, install_cmd
    );

    run_script(vec![&install_cmd], get_worker_dir()).await?;

    let final_cmd = format!(
        "cd {} && {} && {}",
        get_project_path_to_run(run_details).await,
        env_cmd,
        run_cmd
    );

    println!("final command is: {}", final_cmd);

    run_script_bg(vec![&final_cmd], get_worker_dir())?;
    Ok(())
}
