use std::path::PathBuf;

use crate::{
    app_errors::AppError,
    core::{
        app_types::DeployDetails, config::app_config::get_worker_dir, infra::process::run_script,
    },
    shared::github_app::GithubApp,
};

pub async fn pull(
    deploy_details: &DeployDetails,
    commit_hash: Option<String>,
) -> Result<(bool, String, Option<String>), AppError> {
    let (commit_hash, branch) = download_repo(deploy_details, commit_hash).await?;

    extract_project(deploy_details).await?;

    rename_project(deploy_details).await?;

    let shipr_json_exists = move_shipr_json_to_root(deploy_details).await?;

    Ok((shipr_json_exists, commit_hash, Some(branch)))
}

async fn download_repo(
    deploy_details: &DeployDetails,
    commit_hash: Option<String>,
) -> Result<(String, String), AppError> {
    let github_app = GithubApp::new();

    let (owner, repo) = {
        let parts: Vec<&str> = deploy_details.full_name.split('/').collect();
        (parts[0].to_string(), parts[1].to_string())
    };

    let (commit_hash, branch) = if commit_hash.is_none() {
        github_app
            .get_commit_sha(
                deploy_details.branch.clone(),
                &owner,
                &repo,
                &deploy_details.installation_access_token,
            )
            .await?
    } else {
        (commit_hash.unwrap(), deploy_details.branch.clone().unwrap())
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

    run_script(vec![&git_pull_cmd], get_worker_dir()).await?;

    Ok((commit_hash, branch))
}

async fn extract_project(deploy_details: &DeployDetails) -> Result<(), AppError> {
    let extract_cmd = format!("tar -xzf {}.tar.gz", deploy_details.project_id);

    println!("extract command is: {}", extract_cmd);

    run_script(vec![&extract_cmd], get_worker_dir()).await?;

    Ok(())
}

async fn rename_project(deploy_details: &DeployDetails) -> Result<(), AppError> {
    let rename_cmd = format!(
        "mv {}-* {}",
        deploy_details.full_name.replace("/", "-"),
        deploy_details.project_id
    );

    println!("rename command is: {}", rename_cmd);

    run_script(vec![&rename_cmd], get_worker_dir()).await?;

    Ok(())
}

async fn move_shipr_json_to_root(deploy_details: &DeployDetails) -> Result<bool, AppError> {
    let root_dir = deploy_details.root_dir.replace("/", "");
    let shipr_json_path = format!(
        "/root/{}/{}/shipr.json",
        deploy_details.project_id, root_dir
    );

    let path_exists = PathBuf::from(&shipr_json_path).exists();

    if !path_exists {
        return Ok(false);
    }

    let copy_shipr_json = format!("cp {} /root/", shipr_json_path);

    println!("copy shipr json command is: {}", copy_shipr_json);

    run_script(vec![&copy_shipr_json], get_worker_dir()).await?;

    Ok(true)
}
