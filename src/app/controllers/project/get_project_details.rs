use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::projects;
use crate::app_errors::AppError;
use crate::core::config::project_default_config::get_default_config;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

#[derive(serde::Deserialize)]
pub struct GetProjectDetailsQuery {
    pub project_id: i32,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetail {
    pub id: i32,
    pub project_id: String,
    pub full_name: String,
    pub branch: String,
    pub status: String,
    pub last_deployment_time: NaiveDateTime,
    pub root_dir: String,
    pub install_cmds: Vec<String>,
    pub build_cmds: Vec<String>,
    pub run_cmds: Vec<String>,
    pub github_url: String,
    pub commit_hash: String,
}

pub async fn get_project_details_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<GetProjectDetailsQuery>,
) -> Result<HttpResponse, AppError> {
    println!(
        "get_project_details_controller called for project_id: {}",
        query.project_id
    );

    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or_else(|| {
            println!("Error: AuthMiddleware not found in request extensions");
            AppError::InternalServerError
        })?
        .user_id;

    let project_id = query.project_id;

    let row = projects::Entity::find()
        .filter(projects::Column::Id.eq(project_id))
        .filter(projects::Column::UserId.eq(user_id))
        .one(pool.as_ref())
        .await
        .map_err(|e| {
            println!("Database error: {}", e);
            AppError::Database(e.to_string())
        })?
        .ok_or_else(|| {
            println!("Project not found: id={}, user_id={}", project_id, user_id);
            AppError::Database("Project not found".to_string())
        })?;

    let status = match row.status {
        crate::app::models::ProjectStatus::Deploying => "building",
        crate::app::models::ProjectStatus::Running => "running",
        crate::app::models::ProjectStatus::Stopped => "stopped",
        crate::app::models::ProjectStatus::Ready => "ready",
        crate::app::models::ProjectStatus::ErrorStatus => "error",
    };

    let config = get_default_config(&row.project_type);

    let install_cmds = config
        .install_commands
        .into_iter()
        .map(String::from)
        .collect();

    let build_cmds = config
        .build_commands
        .into_iter()
        .map(String::from)
        .collect();

    let run_cmds = config
        .run_commands
        .unwrap_or_default()
        .into_iter()
        .map(String::from)
        .collect();

    let project = ProjectDetail {
        id: row.id,
        project_id: row.project_id,
        full_name: row.full_name,
        branch: row.branch,
        status: status.to_string(),
        last_deployment_time: row
            .last_deployment_time
            .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        root_dir: row.root_dir,
        install_cmds,
        build_cmds,
        run_cmds,
        github_url: row.url.unwrap_or_default(),
        commit_hash: row.commit_hash,
    };

    println!(
        "Successfully fetched project details for id: {}",
        project.id
    );

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Successfully fetched the project details".to_string(),
        data: Some(project),
    }))
}
