use crate::app::db::DbPool;
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
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
    pub home_dir: String,

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

    let query = r#"
        INSERT INTO projects (
            name, description, slug, install_cmds, run_cmds,
            build_cmds, dist_dir, home_dir, url, user_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    "#;

    let result = sqlx::query(query)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.slug)
        .bind(&project.install_cmds)
        .bind(&project.run_cmds)
        .bind(&project.build_cmds)
        .bind(&project.dist_dir)
        .bind(&project.home_dir)
        .bind(&project.url)
        .bind(project.user_id)
        .execute(pool.as_ref())
        .await;

    match result {
        Ok(_) => Ok(HttpResponse::Created().json(AddProjectResponse {
            message: "Project created successfully".to_string(),
        })),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}
