use std::fs::File;
use std::io::Write;

use crate::{
    app_errors::AppError,
    core::{
        app_types::DeployDetails,
        config::{app_config::get_worker_dir, project_default_config::get_default_config},
        infra::{detect::detect_project_type, process::run_script},
    },
};

pub async fn build(deploy_details: &DeployDetails) -> Result<(), AppError> {
    build_project(deploy_details).await?;

    archive_project(deploy_details).await?;

    upload_to_s3(deploy_details).await?;

    Ok(())
}

fn get_project_path(deploy_details: &DeployDetails) -> Result<String, AppError> {
    let repo_name = deploy_details.project_id.to_owned();

    let base_path = format!("/root/{}", repo_name);

    if deploy_details.root_dir == "." {
        return Ok(base_path);
    }

    Ok(format!("{}/{}", base_path, deploy_details.root_dir))
}

fn get_build_cmds(deploy_details: &DeployDetails) -> Result<String, AppError> {
    if deploy_details.build_commands.is_some() {
        Ok(deploy_details.build_commands.as_ref().unwrap().join("&&"))
    } else {
        let project_path = get_project_path(deploy_details)?;

        let project_type = detect_project_type(&project_path);

        let config = get_default_config(&project_type);

        Ok(config.build_commands.join("&&"))
    }
}

async fn build_project(deploy_details: &DeployDetails) -> Result<(), AppError> {
    let project_path = get_project_path(deploy_details)?;

    let build_cmd = format!("cd {} && {}", project_path, get_build_cmds(deploy_details)?);

    run_script(vec![&build_cmd], get_worker_dir()).await?;

    Ok(())
}

async fn archive_project(deploy_details: &DeployDetails) -> Result<String, AppError> {
    let project_path = get_project_path(deploy_details)?;

    let project_type = detect_project_type(&project_path);
    let config = get_default_config(&project_type);

    let mut deploy_details_update = deploy_details.to_owned();
    deploy_details_update.project_type = Some(project_type);

    let mut file = File::options().write(true).open("/root/job.json").unwrap();

    file.write_all(
        serde_json::to_string(&deploy_details_update)
            .unwrap()
            .as_bytes(),
    )?;

    let mkdir_shipr_cmd = "mkdir -p /root/shipr".to_string();

    println!("mkdir shipr command is: {}", mkdir_shipr_cmd);

    let cp_job_json_to_shipr = "cp /root/job.json /root/shipr/job.json".to_string();

    println!("cp job json to shipr command is: {}", cp_job_json_to_shipr);

    let mv_shipr_cmd = format!("mv /root/shipr {}", project_path);

    println!("mv shipr command is: {}", mv_shipr_cmd);

    run_script(
        vec![&mkdir_shipr_cmd, &cp_job_json_to_shipr, &mv_shipr_cmd],
        get_worker_dir(),
    )
    .await?;

    let artifacts = config.deploy_artifacts.join(" ");

    let zip_cmd = format!(
        "cd {} && zip -r /root/{}.zip {} shipr",
        project_path, deploy_details.project_id, artifacts,
    );

    println!("zip command is: {}", zip_cmd);

    run_script(vec![&zip_cmd], get_worker_dir()).await?;

    Ok(format!("/root/{}.zip", deploy_details.project_id))
}

async fn upload_to_s3(deploy_details: &DeployDetails) -> Result<(), AppError> {
    // For dev
    let presigned_upload_url = deploy_details.presigned_upload_url.to_owned().replace(
        "https://francisco-unscholarlike-punctually.ngrok-free.dev/",
        "https://francisco-unscholarlike-punctually.ngrok-free.dev/s3/",
    );

    let upload_cmd = format!(
        "curl -X PUT -T {}.zip '{}'",
        deploy_details.project_id, presigned_upload_url
    );

    run_script(vec![&upload_cmd], get_worker_dir()).await?;

    Ok(())
}
