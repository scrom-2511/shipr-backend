use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    app_errors::AppError, core::app_types::DeployReq, core::config::app_config::get_dir,
    core::infra::process::run_script,
};

#[derive(Deserialize)]
struct DeployResponse {
    project_id: String,
}

pub async fn deploy(deploy_req: DeployReq) -> Result<(), AppError> {
    let github_app_url = "https://github.com/apps/shipr-deployment/installations/new";

    println!(
        "Connect the project's repository with shipr by visiting. Opening the url in your browser..."
    );

    run_script(vec![&format!("xdg-open {}", github_app_url)], get_dir()).await?;

    println!("Waiting for installation...");

    let client = Client::new();

    let res = client
        .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/deploy")
        .json(&deploy_req)
        .send()
        .await?;

    println!("Deploy response: {}", res.status());

    let deploy_response = res.json::<DeployResponse>().await?;

    println!("Project ID: {}", deploy_response.project_id);

    println!("Waiting for queue...");

    let logs_url = format!(
        "https://francisco-unscholarlike-punctually.ngrok-free.dev/logs/{}",
        deploy_response.project_id
    );

    let res = client.get(logs_url).send().await?;

    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;

        let text = String::from_utf8_lossy(&chunk);

        let text = text.replace("data: ", "");

        print!("{}", text);
    }

    Ok(())
}
