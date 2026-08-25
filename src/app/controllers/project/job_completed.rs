use crate::app::db::DbPool;
use crate::app::models::projects;
use crate::app_errors::AppError;
use crate::core::app_types::{JobCompletedReq, JobType};
use actix_web::{HttpResponse, web};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

pub async fn job_completed_controller(
    pool: web::Data<DbPool>,
    body: web::Json<JobCompletedReq>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    let now = chrono::Utc::now().naive_utc();

    println!(
        "Job completed signal received for project: {}, job_type: {:?}",
        body.project_id, body
    );

    let project = projects::Entity::find()
        .filter(projects::Column::ProjectId.eq(&body.project_id))
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| {
            AppError::Database(format!(
                "No project found with project_id: {}",
                body.project_id
            ))
        })?;

    let mut active_project: projects::ActiveModel = project.into();
    active_project.last_deployment_time = Set(Some(now));
    active_project.project_type = Set(body.project_type);
    active_project.status = Set(crate::app::models::ProjectStatus::Running);

    if body.job_type == JobType::Deploy || body.job_type == JobType::Redeploy {
        if let Some(hash) = body.commit_hash {
            active_project.commit_hash = Set(hash);
        }
        if let Some(b) = body.branch {
            active_project.branch = Set(b);
        }
    }

    active_project
        .update(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(HttpResponse::Ok().json(json!({"message": "Project status updated"})))
}
