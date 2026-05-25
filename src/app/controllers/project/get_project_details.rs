use actix_web::{
    HttpResponse,
    web::{self, Query},
};

use crate::{
    app::{controllers::ApiResponse, db::DbPool, models::Project},
    app_errors::AppError,
};

pub struct GetProjectsDetailsQuery {
    pub project_id: i32,
}

pub async fn get_project_details_controller(
    pool: web::Data<DbPool>,
    query: Query<GetProjectsDetailsQuery>,
) -> Result<HttpResponse, AppError> {
    let project_id = query.project_id;

    let query = "SELECT * FROM projects WHERE id = $1";

    let project_details = sqlx::query_as::<_, Project>(query)
        .bind(project_id)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|_e| AppError::InternalServerError)?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Successfully fetched the project details".to_string(),
        data: Some(project_details),
    }))
}
