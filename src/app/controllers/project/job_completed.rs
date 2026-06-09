use crate::app::db::DbPool;
use crate::app_errors::AppError;
use crate::core::app_types::{JobCompletedReq, JobType};
use actix_web::{HttpResponse, web};
use serde_json::json;

pub async fn job_completed_controller(
    pool: web::Data<DbPool>,
    body: web::Json<JobCompletedReq>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    let now = chrono::Utc::now();

    println!(
        "Job completed signal received for project: {}, job_type: {:?}",
        body.project_id, body
    );

    let result = if body.job_type == JobType::Deploy || body.job_type == JobType::Redeploy {
        sqlx::query(
            "UPDATE projects SET commit_hash = $1, last_deployment_time = $2, project_type = $3, branch = $5, status = $6 WHERE project_id = $4",
        )
        .bind(body.commit_hash.unwrap())
        .bind(now)
        .bind(body.project_type.to_string())
        .bind(&body.project_id)
        .bind(body.branch.unwrap())
        .bind("active")
        .execute(pool.as_ref())
        .await
    } else {
        sqlx::query(
            "UPDATE projects SET last_deployment_time = $1, project_type = $2, status = $3 WHERE project_id = $4",
        )
        .bind(now)
        .bind(body.project_type.to_string())
        .bind(&body.project_id)
        .bind("active")
        .execute(pool.as_ref())
        .await
    };

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return Err(AppError::Database(format!(
                    "No project found with project_id: {}",
                    body.project_id
                )));
            }
            Ok(HttpResponse::Ok().json(json!({"message": "Project status updated"})))
        }
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}
