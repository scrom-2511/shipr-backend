use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::projects;
use crate::app_errors::AppError;
use crate::core::app_types::ProjectType;
use crate::core::config::project_default_config::get_default_config;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
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

    println!("user_id: {}", user_id);

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

    let status = match row.status.as_str() {
        "active" => "active",
        "deploying" => "building",
        "error" => "error",
        _ => "error",
    };

    let p_type = row.project_type.unwrap_or(ProjectType::Unknown);

    let install_cmds = if row.install_cmds.is_none() {
        let install_cmds = get_default_config(&p_type)
            .install_commands
            .into_iter()
            .map(String::from)
            .collect();
        Some(install_cmds)
    } else {
        row.install_cmds
    };

    let build_cmds = if row.build_cmds.is_none() {
        let build_cmds = get_default_config(&p_type)
            .build_commands
            .into_iter()
            .map(String::from)
            .collect();
        Some(build_cmds)
    } else {
        row.build_cmds
    };

    let run_cmds = if row.run_cmds.is_none() {
        let run_cmds = get_default_config(&p_type)
            .run_commands
            .unwrap_or_default()
            .into_iter()
            .map(String::from)
            .collect();
        Some(run_cmds)
    } else {
        row.run_cmds
    };

    let project = ProjectDetail {
        id: row.id,
        project_id: row.project_id,
        full_name: row.full_name,
        branch: row.branch.unwrap_or_default(),
        status: status.to_string(),
        last_deployment_time: row
            .last_deployment_time
            .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        root_dir: row.root_dir,
        install_cmds: install_cmds.unwrap_or_default(),
        build_cmds: build_cmds.unwrap_or_default(),
        run_cmds: run_cmds.unwrap_or_default(),
        github_url: row.url.unwrap_or_default(),
        commit_hash: row.commit_hash.unwrap_or_default(),
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
