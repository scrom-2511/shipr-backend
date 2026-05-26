use crate::app::db::DbPool;
use crate::app_errors::AppError;
use crate::core::app_types::{JobCompletedReq, JobType};
use actix_web::{HttpResponse, web};
use chrono::Utc;
use serde_json::json;

pub async fn job_completed_controller(
    pool: web::Data<DbPool>,
    body: web::Json<JobCompletedReq>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();
    let now = Utc::now();

    println!(
        "Job completed signal received for project: {}, job_type: {:?}",
        req.project_id, req.job_type
    );

    let result = if req.job_type == JobType::Deploy {
        sqlx::query(
            "UPDATE projects SET commit_hash = $1, last_deployment_time = $2, project_type = $3 WHERE full_name = $4",
        )
        .bind(req.commit_hash)
        .bind(now)
        .bind(req.project_type.to_string())
        .bind(&req.project_id)
        .execute(pool.as_ref())
        .await
    } else {
        sqlx::query("UPDATE projects SET last_deployment_time = $1, project_type = $2 WHERE full_name = $3")
            .bind(now)
            .bind(req.project_type.to_string())
            .bind(&req.project_id)
            .execute(pool.as_ref())
            .await
    };

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return Err(AppError::Database(format!(
                    "No project found with full_name: {}",
                    req.project_id
                )));
            }
            Ok(HttpResponse::Ok().json(json!({"message": "Project status updated"})))
        }
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}
