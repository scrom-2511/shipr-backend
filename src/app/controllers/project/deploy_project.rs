use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{self},
};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::{
    app::{controllers::ApiResponse, db::DbPool, middlewares::AuthMiddleware},
    app_errors::AppError,
    core::{app_types::DeployReq, controller::queue::deploy_queue::DeployQueue},
};

#[derive(Serialize, Deserialize, FromRow)]
struct InstallationIds {
    installation_ids: Vec<i32>,
}

pub async fn deploy_project_controller(
    body: web::Json<DeployReq>,
    deploy_queue: web::Data<DeployQueue>,
    // logs_store: LogsStore,
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();

    println!("Deploy details: {:?}", body);

    let project_id = body.full_name.replace("/", "~");

    // let (tx, _) = channel::<String>(100);

    // let file_path = "/home/scrom/code/shipr/logs";

    // fs::create_dir_all(file_path).unwrap();

    // fs::File::create(format!("{}/{}.txt", file_path, project_id)).unwrap();

    // logs_store.lock().await.insert(project_id.clone(), tx);

    // println!("{:?}", logs_store.lock().await.keys());

    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let query_to_check_installation_id =
        "SELECT installation_ids FROM github_repos WHERE user_id = $1";

    let installation_id: Option<InstallationIds> = sqlx::query_as(query_to_check_installation_id)
        .bind(user_id)
        .fetch_optional(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if installation_id.is_none() {
        return Err(AppError::Database("Installation id not found".to_string()));
    }

    let installation_ids = installation_id.unwrap().installation_ids;

    let exists = installation_ids.contains(&(body.installation_id as i32));

    if !exists {
        return Err(AppError::Database("Installation id not found".to_string()));
    }

    deploy_queue.add_to_queue(&body).await?;

    let query = "INSERT INTO projects (name, status, last_deployment_time, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)";

    sqlx::query(query)
        .bind(&body.name)
        .bind("deploying")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Project added to queue successfully".to_string(),
        data: None,
    }))
}
