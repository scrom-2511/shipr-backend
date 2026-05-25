use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;

use crate::{
    app::{
        controllers::{ApiResponse, auth::decode_token},
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

    let query = r#"UPDATE github_repos SET user_id = $1 WHERE $2 = ANY(installation_ids)"#;

    let execute = sqlx::query(query)
        .bind(user_id)
        .bind(body.installation_id)
        .execute(pool.as_ref())
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "User ID updated successfully".to_string(),
        data: None,
    }))
}
