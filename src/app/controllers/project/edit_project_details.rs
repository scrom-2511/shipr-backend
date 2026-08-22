use actix_web::{web, HttpResponse};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;

use crate::{
    app::{controllers::ApiResponse, db::DbPool, models::projects},
    app_errors::AppError,
};

#[derive(Debug, Deserialize)]
pub struct EditProjectBody {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub root_dir: String,
    pub dist_dir: String,
    pub install_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
}

pub async fn edit_project_details_controller(
    pool: web::Data<DbPool>,
    body: web::Json<EditProjectBody>,
) -> Result<HttpResponse, AppError> {
    let project = projects::Entity::find_by_id(body.id)
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Database("Project not found".to_string()))?;

    let mut active_project: projects::ActiveModel = project.into();
    active_project.project_id = Set(body.name.clone());
    active_project.url = Set(Some(body.url.clone()));
    active_project.branch = Set(Some(body.branch.clone()));
    active_project.root_dir = Set(body.root_dir.clone());
    active_project.dist_dir = Set(Some(body.dist_dir.clone()));
    active_project.install_cmds = Set(body.install_cmds.clone());
    active_project.build_cmds = Set(body.build_cmds.clone());
    active_project.run_cmds = Set(body.run_cmds.clone());

    active_project
        .update(pool.as_ref())
        .await
        .map_err(|e| {
            println!("DB ERROR: {:?}", e);
            AppError::Database(e.to_string())
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Successfully updated the project details".to_string(),
        data: None,
    }))
}
