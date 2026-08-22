use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::projects;
use crate::app_errors::AppError;

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct DeployedProject {
    id: i32,
    project_id: String,
    branch: String,
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

    let rows = projects::Entity::find()
        .filter(projects::Column::UserId.eq(user_id))
        .order_by_desc(projects::Column::CreatedAt)
        .all(pool.as_ref())
        .await
        .map_err(|e| {
            println!("Database error in get_all_deployed_projects: {}", e);
            AppError::Database(e.to_string())
        })?;

    let projects: Vec<DeployedProject> = rows
        .into_iter()
        .map(|row| DeployedProject {
            id: row.id,
            project_id: row.project_id,
            branch: row.branch.unwrap_or_default(),
            full_name: row.full_name,
            status: row.status,
            last_deployment_time: row
                .last_deployment_time
                .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Projects fetched successfully".to_string(),
        data: Some(GetAllProjectsResponse { projects }),
    }))
}
