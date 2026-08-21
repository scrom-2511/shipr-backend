use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;
use crate::core::config::project_default_config::get_default_config;
use crate::{app::controllers::ApiResponse, core::app_types::ProjectType};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;

#[derive(serde::Deserialize)]
pub struct GetProjectDetailsQuery {
    pub project_id: i32,
}

#[derive(Debug, Serialize, FromRow)]
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

#[derive(FromRow)]
struct ProjectRow {
    pub id: i32,
    pub project_id: String,
    pub full_name: String,
    pub branch: Option<String>,
    pub status: String,
    pub last_deployment_time: Option<NaiveDateTime>,
    pub root_dir: String,
    pub install_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
    pub url: Option<String>,
    pub commit_hash: Option<String>,
    pub project_type: Option<ProjectType>,
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

    let query_str = r#"
        SELECT 
            id, project_id, full_name, branch, status::text as status, last_deployment_time, 
            root_dir, dist_dir, install_cmds, build_cmds, run_cmds, 
            url, commit_hash, project_type
        FROM projects 
        WHERE id = $1 AND user_id = $2
    "#;

    let row: ProjectRow = sqlx::query_as(query_str)
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool.as_ref())
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

    let install_cmds = if row.install_cmds.is_none() {
        let install_cmds = get_default_config(&row.project_type.unwrap())
            .install_commands
            .into_iter()
            .map(String::from)
            .collect();
        Some(install_cmds)
    } else {
        row.install_cmds
    };

    let build_cmds = if row.build_cmds.is_none() {
        let build_cmds = get_default_config(&row.project_type.unwrap())
            .build_commands
            .into_iter()
            .map(String::from)
            .collect();
        Some(build_cmds)
    } else {
        row.build_cmds
    };

    let run_cmds = if row.run_cmds.is_none() {
        let run_cmds = get_default_config(&row.project_type.unwrap())
            .run_commands
            .unwrap()
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
        install_cmds: install_cmds.unwrap(),
        build_cmds: build_cmds.unwrap(),
        run_cmds: run_cmds.unwrap(),
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
