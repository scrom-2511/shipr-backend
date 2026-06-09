use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DeleteProjectQuery {
    pub project_id: i32,
}

pub async fn delete_project_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<DeleteProjectQuery>,
) -> Result<HttpResponse, AppError> {
    println!("delete_project_controller called for project_id: {}", query.project_id);

    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or_else(|| {
            println!("Error: AuthMiddleware not found in request extensions");
            AppError::InternalServerError
        })?
        .user_id;

    let project_id = query.project_id;

    let result = sqlx::query("DELETE FROM projects WHERE id = $1 AND user_id = $2")
        .bind(project_id)
        .bind(user_id)
        .execute(pool.as_ref())
        .await
        .map_err(|e| {
            println!("Database error during deletion: {}", e);
            AppError::Database(e.to_string())
        })?;

    if result.rows_affected() == 0 {
        println!("Project not found or permission denied: id={}, user_id={}", project_id, user_id);
        return Err(AppError::Database("Project not found or you don't have permission to delete it".to_string()));
    }

    println!("Successfully deleted project id: {}", project_id);

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Project deleted successfully".to_string(),
        data: None,
    }))
}
