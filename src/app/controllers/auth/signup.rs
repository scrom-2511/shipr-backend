use crate::app::models::users;
use crate::app::state::AppState;
use crate::app::{controllers::ApiResponse, db::DbPool};
use crate::app_errors::AppError;
use actix_web::{HttpResponse, web};
use dodopayments::models::CustomersCreateParams;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct SignupRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub username: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponse {
    pub message: String,
}

pub async fn signup_controller(
    state: web::Data<AppState>,
    pool: web::Data<DbPool>,
    body: web::Json<SignupRequest>,
) -> Result<HttpResponse, AppError> {
    let signup = body.into_inner();

    println!("{:?}", signup);

    signup
        .validate()
        .map_err(|err| AppError::ValidationError(err.to_string()))?;

    let hashed_password =
        bcrypt::hash(&signup.password, 10).map_err(|_| AppError::InternalServerError)?;

    let mut new_user = users::ActiveModel {
        username: Set(signup.username.clone()),
        email: Set(signup.email.clone()),
        password: Set(hashed_password),
        ..Default::default()
    };

    let customer = state
        .client
        .customers()
        .create()
        .body(CustomersCreateParams {
            email: Some(signup.email),
            name: Some(signup.username),
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::DodoError(e.to_string()))?;

    new_user.dodo_customer_id = Set(Some(customer.customer_id));
    new_user.insert(pool.as_ref()).await?;

    Ok(HttpResponse::Created().json(ApiResponse::<()> {
        success: true,
        message: "User created successfully".to_string(),
        data: None,
    }))
}
