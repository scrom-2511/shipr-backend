use actix_web::web;
use serde::{Deserialize, Serialize};

use crate::{
    app::db::DbPool,
    app_errors::AppError,
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum GithubWebhookAction {
    Created,
    Deleted,
}

#[derive(Debug, Deserialize, Serialize)]
struct GithubAccount {
    login: String,
    id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct GithubAppInstallation {
    id: i32,
    client_id: String,
    account: GithubAccount,
}

#[derive(Debug, Deserialize)]
pub struct GithubInstallationRepositoriesResponse {
    pub repositories: Vec<GithubRepository>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GithubRepository {
    pub id: i32,
    pub name: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GithubAppWebhookPayload {
    action: GithubWebhookAction,
    installation: GithubAppInstallation,

    #[serde(default)]
    repositories: Vec<GithubRepository>,
}

pub async fn github_webhook_installation(
    body: web::Json<GithubAppWebhookPayload>,
    pool: web::Data<DbPool>,
) -> Result<(), AppError> {
    let body = body.into_inner();

    let installation_id = vec![body.installation.id];

    println!("installation_id: {:?}", installation_id);

    let query = r#"INSERT INTO github_repos (installation_ids) VALUES ($1)"#;

    sqlx::query(query)
        .bind(installation_id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(())
}
