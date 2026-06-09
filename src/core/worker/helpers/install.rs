use crate::{
    app_errors::AppError,
    core::{
        app_types::{DeployDetails, ShiprJson},
        config::{app_config::get_worker_dir, project_default_config::get_default_config},
        infra::{detect::detect_project_type, process::run_script},
    },
};

pub async fn install(
    deploy_details: &DeployDetails,
    shipr_json: &ShiprJson,
) -> Result<(), AppError> {
    let project_path = get_project_path(deploy_details)?;

    let install_cmd = format!(
        "cd {} && {}",
        project_path,
        get_install_cmds(shipr_json, &project_path)?
    );

    run_script(vec![&install_cmd], get_worker_dir()).await?;

    Ok(())
}

pub fn get_install_cmds(shipr_json: &ShiprJson, project_path: &str) -> Result<String, AppError> {
    if shipr_json.install_commands.is_some() {
        Ok(shipr_json.install_commands.as_ref().unwrap().join("&&"))
    } else {
        let project_type = detect_project_type(project_path);

        let config = get_default_config(&project_type);

        Ok(config.install_commands.join("&&"))
    }
}

fn get_project_path(deploy_details: &DeployDetails) -> Result<String, AppError> {
    let repo_name = deploy_details.project_id.to_owned();

    let base_path = format!("/root/{}", repo_name);

    if deploy_details.root_dir == "." {
        Ok(base_path)
    } else {
        Ok(format!("{}/{}", base_path, deploy_details.root_dir))
    }
}
