use crate::app::db::DbPool;
use crate::app::models::projects;
use crate::app_errors::AppError;
use actix_web::{web, HttpResponse};
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct AddProjectRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

    pub description: Option<String>,

    #[validate(length(min = 1, message = "Slug is required"))]
    pub slug: String,

    pub install_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,

    #[validate(length(min = 1, message = "Dist directory is required"))]
    pub dist_dir: String,

    #[validate(length(min = 1, message = "Home directory is required"))]
    pub root_dir: String,

    #[validate(length(min = 1, message = "URL is required"))]
    pub url: String,

    pub user_id: i32,
}

#[derive(Debug, Serialize)]
pub struct AddProjectResponse {
    pub message: String,
}

pub async fn add_new_project(
    pool: web::Data<DbPool>,
    body: web::Json<AddProjectRequest>,
) -> Result<HttpResponse, AppError> {
    let project = body.into_inner();

    project
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let new_project = projects::ActiveModel {
        full_name: Set(project.name),
        project_id: Set(project.slug),
        install_cmds: Set(project.install_cmds),
        run_cmds: Set(project.run_cmds),
        build_cmds: Set(project.build_cmds),
        dist_dir: Set(Some(project.dist_dir)),
        root_dir: Set(project.root_dir),
        url: Set(Some(project.url)),
        user_id: Set(project.user_id),
        status: Set("active".to_string()),
        ..Default::default()
    };

    match new_project.insert(pool.as_ref()).await {
        Ok(_) => Ok(HttpResponse::Created().json(AddProjectResponse {
            message: "Project created successfully".to_string(),
        })),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}
