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
) -> Result<(String, Option<String>), AppError> {
    let (commit_hash, branch) = download_repo(deploy_details, commit_hash).await?;

    extract_project(deploy_details).await?;

    rename_project(deploy_details).await?;

    Ok((commit_hash, Some(branch)))
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
