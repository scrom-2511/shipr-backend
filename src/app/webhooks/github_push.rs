use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};

use crate::{
    app::{controllers::ApiResponse, db::DbPool},
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

#[derive(Serialize, Deserialize, Clone)]
pub struct GithubPushEvent {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub after: String,
    pub repository: Repository,
    pub installation: Installation,
}

pub async fn github_webhook_push(
    body: web::Json<GithubPushEvent>,
    pool: web::Data<DbPool>,
    redeploy_queue: web::Data<ReDeployQueue>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();

    redeploy_queue.publish(&body).await.unwrap();

    let query = r#"
        UPDATE projects 
        SET commit_hash = $1, updated_at = $2 
        WHERE full_name = $3
    "#;

    sqlx::query(query)
        .bind(&body.after)
        .bind(chrono::Utc::now())
        .bind(&body.repository.full_name)
        .execute(pool.as_ref())
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Installation recorded successfully".to_string(),
        data: None,
    }))
}
