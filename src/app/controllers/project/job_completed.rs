use crate::app::db::DbPool;
use crate::app_errors::AppError;
use crate::core::app_types::{JobCompletedReq, JobType};
use actix_web::{HttpResponse, web};
use serde_json::json;

use crate::core::config::project_default_config::get_default_config;

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

    let default_config = if body.project_type != crate::core::app_types::ProjectType::Unknown {
        Some(get_default_config(body.project_type.clone()))
    } else {
        None
    };

    let default_install = default_config.as_ref().map(|c| {
        c.install_commands
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<_>>()
    });
    let default_build = default_config.as_ref().map(|c| {
        c.build_commands
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let default_run = default_config
        .as_ref()
        .and_then(|c| c.run_command.map(|s| vec![s.to_string()]));

    let result = if body.job_type == JobType::Deploy || body.job_type == JobType::Redeploy {
        sqlx::query(
            "UPDATE projects SET commit_hash = $1, last_deployment_time = $2, project_type = $3, branch = COALESCE(branch, $5), install_cmds = COALESCE(install_cmds, $6), build_cmds = COALESCE(build_cmds, $7), run_cmds = COALESCE(run_cmds, $8), status = $9 WHERE project_id = $4",
        )
        .bind(body.commit_hash.unwrap())
        .bind(now)
        .bind(body.project_type.to_string())
        .bind(&body.project_id)
        .bind(body.branch.unwrap())
        .bind(&body.project_id)
        .bind(default_install)
        .bind(default_build)
        .bind(default_run)
        .bind("active")
        .execute(pool.as_ref())
        .await
    } else {
        sqlx::query(
            "UPDATE projects SET last_deployment_time = $1, project_type = $2, install_cmds = COALESCE(install_cmds, $4), build_cmds = COALESCE(build_cmds, $5), run_cmds = COALESCE(run_cmds, $6), status = $7 WHERE project_id = $3",
        )
        .bind(now)
        .bind(body.project_type.to_string())
        .bind(&body.project_id)
        .bind(default_install)
        .bind(default_build)
        .bind(default_run)
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
