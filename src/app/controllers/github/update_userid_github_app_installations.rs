use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseBackend, EntityTrait, QueryFilter, Set, Statement,
    sea_query::Expr,
};
use serde::Deserialize;

use crate::{
    app::{
        controllers::{ApiResponse, auth::decode_token},
        db::DbPool,
        middlewares::AuthMiddleware,
        models::github_repos,
    },
    app_errors::AppError,
};

#[derive(Debug, Deserialize)]
pub struct UpdateUserIdGithubAppInstallationsRequest {
    pub installation_id: i32,
    pub state: String,
}

pub async fn update_userid_github_app_installations(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<UpdateUserIdGithubAppInstallationsRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    println!("hi there 1");

    let decoded_state = decode_token(&body.state)?;

    println!("hi there 2");

    if decoded_state.user_id != user_id {
        return Err(AppError::InvalidCredentials);
    }

    println!("hi there 3");

    let repo = match github_repos::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT * FROM github_repos WHERE $1 = ANY(installation_ids)",
            vec![body.installation_id.into()],
        ))
        .one(pool.as_ref())
        .await
    {
        Ok(repo) => repo,
        Err(e) => {
            println!("Installation id update failed: {}", e);
            return Err(AppError::Database(e.to_string()));
        }
    };

    let repo = repo.ok_or_else(|| AppError::Database("Installation not found".to_string()))?;

    let mut active_repo: github_repos::ActiveModel = repo.into();

    active_repo.user_id = Set(Some(user_id));

    match active_repo.update(pool.as_ref()).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "User ID updated successfully".to_string(),
            data: None,
        })),
        Err(e) => {
            println!("Installation id update failed: {}", e);
            Err(AppError::Database(e.to_string()))
        }
    }
}
