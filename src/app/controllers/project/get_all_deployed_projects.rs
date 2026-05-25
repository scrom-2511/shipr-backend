use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
struct DeployedProject {
    id: i32,
    branch: String,
    name: String,
    full_name: String,
    status: String,
    last_deployment_time: NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct GetAllProjectsResponse {
    projects: Vec<DeployedProject>,
}

pub async fn get_all_deployed_projects_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    println!("get_all_deployed_projects_controller called");

    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    println!("user_id: {}", user_id);

    let query = r#"
    SELECT
        id,
        branch,
        name,
        full_name,
        status,
        last_deployment_time
    FROM projects
    WHERE user_id = $1
    ORDER BY created_at DESC
    "#;

    let projects: Vec<DeployedProject> = sqlx::query_as(query)
        .bind(user_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Projects fetched successfully".to_string(),
        data: Some(GetAllProjectsResponse { projects }),
    }))
}
