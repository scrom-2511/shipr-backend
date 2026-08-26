use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{self},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    app::{
        controllers::ApiResponse,
        db::DbPool,
        middlewares::AuthMiddleware,
        models::{github_repos, projects},
    },
    app_errors::AppError,
    core::{app_types::DeployReq, controller::queue::deploy_queue::DeployQueue},
};

pub async fn deploy_project_controller(
    body: web::Json<DeployReq>,
    deploy_queue: web::Data<DeployQueue>,
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();

    println!("Deploy details: {:?}", body);

    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let installation_repo = match github_repos::Entity::find()
        .filter(github_repos::Column::UserId.eq(user_id))
        .one(pool.as_ref())
        .await
    {
        Ok(Some(repo)) => repo,
        Ok(None) => return Err(AppError::Database("Installation id not found".to_string())),
        Err(e) => return Err(AppError::Database(e.to_string())),
    };

    let installation_id = body.installation_id as i32;

    let contains_installation = installation_repo
        .installation_ids
        .as_ref()
        .map(|ids| ids.contains(&installation_id))
        .unwrap_or(false);

    if !contains_installation {
        return Err(AppError::Database("Installation id not found".to_string()));
    }

    deploy_queue.add_to_queue(&body).await?;

    println!("Project added to queue successfully");

    let envs = {
        let json = serde_json::to_string(&body.envs).unwrap();
        let encrypted = crate::shared::crypto::Crypto::encrypt(&json);

        serde_json::Value::String(encrypted)
    };

    let now = chrono::Utc::now().naive_utc();

    let new_project = projects::ActiveModel {
        project_id: Set(body.project_id.clone()),
        status: Set(crate::app::models::ProjectStatus::Deploying),
        root_dir: Set(body.root_dir.clone()),
        full_name: Set(body.full_name.clone()),
        user_id: Set(user_id),
        last_deployment_time: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        branch: Set(body.branch),
        envs: Set(Some(envs)),
        ..Default::default()
    };

    match new_project.insert(pool.as_ref()).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "Project added to queue successfully".to_string(),
            data: None,
        })),
        Err(e) => {
            println!("Error in creating new project: {}", e);
            return Err(AppError::Database(e.to_string()));
        }
    }
}
