use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};

use crate::{
    app::{controllers::ApiResponse, db::DbPool},
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
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM projects
            WHERE name = $1
        )
        "#,
    )
    .bind(&body.project_name)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| {
        println!("DB ERROR: {:?}", e);
        AppError::InternalServerError
    })?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Project name availability checked successfully".to_string(),
        data: Some(CheckNameAvailabilityResponse {
            is_available: !exists,
        }),
    }))
}
