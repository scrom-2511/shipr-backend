use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;

use crate::{
    app::{
        controllers::{auth::decode_token, ApiResponse},
        db::DbPool,
        middlewares::AuthMiddleware,
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
    println!("body: {:?}", body);
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let decoded_state = decode_token(&body.state)?;

    if decoded_state.user_id != user_id {
        return Err(AppError::InvalidCredentials);
    }

    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE github_repos SET user_id = $1 WHERE $2 = ANY(installation_ids)",
        vec![user_id.into(), body.installation_id.into()],
    );

    pool.execute(stmt)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "User ID updated successfully".to_string(),
        data: None,
    }))
}
