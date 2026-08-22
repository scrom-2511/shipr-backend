use actix_web::{HttpResponse, web};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use crate::{
    app::controllers::auth::generate_token, app::db::DbPool, app::models::users,
    app_errors::AppError,
};

const GITHUB_CLIENT_ID: &str = "YOUR_GITHUB_CLIENT_ID";
const GITHUB_CLIENT_SECRET: &str = "YOUR_GITHUB_CLIENT_SECRET";
const GITHUB_REDIRECT_URI: &str = "http://localhost:3000/auth/github/callback";

#[derive(Debug, Serialize)]
pub struct GithubAuthUrlResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubCallbackRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct GithubSignupResponse {
    pub message: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct GithubUserResponse {
    id: i64,
    login: String,
    email: Option<String>,
    name: Option<String>,
}

pub async fn github_auth_url() -> Result<HttpResponse, AppError> {
    let scope = "read:user user:email";
    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}",
        GITHUB_CLIENT_ID, GITHUB_REDIRECT_URI, scope
    );

    Ok(HttpResponse::Ok().json(GithubAuthUrlResponse { url: auth_url }))
}

pub async fn github_callback(
    pool: web::Data<DbPool>,
    query: web::Query<GithubCallbackRequest>,
) -> Result<HttpResponse, AppError> {
    let code = &query.code;

    let token_url = "https://github.com/login/oauth/access_token";
    let client = reqwest::Client::new();

    let form_data = format!(
        "client_id={}&client_secret={}&code={}",
        GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, code
    );

    let token_response: GithubTokenResponse = client
        .post(token_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_data)
        .send()
        .await
        .map_err(|e| AppError::GithubOAuthError(e.to_string()))?
        .json()
        .await
        .map_err(|e| AppError::GithubOAuthError(e.to_string()))?;

    let user_response: GithubUserResponse = client
        .get("https://api.github.com/user")
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token),
        )
        .header("User-Agent", "Shipr-App")
        .send()
        .await
        .map_err(|e| AppError::GithubOAuthError(e.to_string()))?
        .json()
        .await
        .map_err(|e| AppError::GithubOAuthError(e.to_string()))?;

    let email = if let Some(email) = user_response.email {
        email
    } else {
        let emails: Vec<serde_json::Value> = client
            .get("https://api.github.com/user/emails")
            .header(
                "Authorization",
                format!("Bearer {}", token_response.access_token),
            )
            .header("User-Agent", "Shipr-App")
            .send()
            .await
            .map_err(|e| AppError::GithubOAuthError(e.to_string()))?
            .json()
            .await
            .map_err(|e| AppError::GithubOAuthError(e.to_string()))?;

        emails
            .into_iter()
            .find(|e| e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false))
            .and_then(|e| {
                e.get("email")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| AppError::GithubOAuthError("No email found".to_string()))?
    };

    let name = user_response
        .name
        .unwrap_or_else(|| user_response.login.clone());

    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let user_id = if let Some(user) = existing_user {
        user.id
    } else {
        let new_user = users::ActiveModel {
            username: Set(name),
            email: Set(email),
            password: Set("github_oauth".to_string()),
            ..Default::default()
        };
        let res = new_user
            .insert(pool.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        res.id
    };

    let token = generate_token(user_id)?;

    Ok(HttpResponse::Ok().json(GithubSignupResponse {
        message: "GitHub signup successful".to_string(),
        token,
    }))
}
