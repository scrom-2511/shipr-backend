use actix_web::web;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    app::{db::DbPool, models::projects},
    app_errors::AppError,
    core::controller::queue::redeploy_queue::ReDeployQueue,
};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct Installation {
    pub id: u64,
}

#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Commit {
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GithubPushEvent {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub after: String,
    pub repository: Repository,
    pub installation: Installation,
    pub commits: Vec<Commit>,
}

pub async fn github_webhook_push(
    body: web::Json<GithubPushEvent>,
    pool: web::Data<DbPool>,
    redeploy_queue: web::Data<ReDeployQueue>,
) -> Result<(), AppError> {
    println!("github_webhook_push called");
    let body = body.into_inner();

    let incoming_branch = body.ref_field.replace("refs/heads/", "");
    let full_name = &body.repository.full_name;

    let project_exists = projects::Entity::find()
        .filter(projects::Column::FullName.eq(full_name))
        .filter(projects::Column::Branch.eq(&incoming_branch))
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .is_some();

    if project_exists {
        redeploy_queue.publish(&body).await?;
    }

    Ok(())
}
