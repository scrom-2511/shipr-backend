use actix_web::{cookie::SameSite, web, HttpResponse};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    app::{
        controllers::{auth::generate_token, ApiResponse},
        db::DbPool,
        models::users,
    },
    app_errors::AppError,
};

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct SigninRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

pub async fn signin_controller(
    pool: web::Data<DbPool>,
    body: web::Json<SigninRequest>,
) -> Result<HttpResponse, AppError> {
    let signin = body.into_inner();

    println!("{:?}", signin);

    signin
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    println!("Valid credentials");

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&signin.email))
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or(AppError::UserNotFound)?;

    println!("User found");

    let is_valid = bcrypt::verify(&signin.password, &user.password)
        .map_err(|_| AppError::PasswordHashFailed)?;

    println!("Password verified");

    if !is_valid {
        println!("Invalid credentials");
        return Err(AppError::InvalidCredentials);
    }

    let token = generate_token(user.id)?;

    Ok(HttpResponse::Ok()
        .cookie(
            actix_web::cookie::Cookie::build("token", token)
                .path("/")
                .http_only(true)
                .secure(true)
                .same_site(SameSite::None)
                .finish(),
        )
        .json(ApiResponse::<()> {
            success: true,
            message: "Login successful".to_string(),
            data: None,
        }))
}
