use actix_web::{web, HttpResponse};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    app::{controllers::ApiResponse, db::DbPool, models::projects},
    app_errors::AppError,
};

#[derive(Deserialize)]
pub struct CheckNameAvailabilityRequest {
    project_name: String,
}

#[derive(Serialize)]
pub struct CheckNameAvailabilityResponse {
    is_available: bool,
}

pub async fn check_repo_name_availability(
    body: web::Json<CheckNameAvailabilityRequest>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    println!("check_repo_name_availability called");
    let exists = projects::Entity::find()
        .filter(projects::Column::ProjectId.eq(&body.project_name))
        .one(pool.as_ref())
        .await
        .map_err(|e| {
            println!("DB ERROR: {:?}", e);
            AppError::Database(e.to_string())
        })?
        .is_some();

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Project name availability checked successfully".to_string(),
        data: Some(CheckNameAvailabilityResponse {
            is_available: !exists,
        }),
    }))
}
