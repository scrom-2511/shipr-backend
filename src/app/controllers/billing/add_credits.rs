// use crate::app::controllers::ApiResponse;
// use crate::app::db::DbPool;
// use crate::app::middlewares::AuthMiddleware;
// use crate::app::models::users;
// use crate::app_errors::AppError;

// use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
// use sea_orm::{ActiveModelTrait, EntityTrait, Set};
// use serde::{Deserialize, Serialize};
// use validator::Validate;

// #[derive(Debug, Deserialize, Validate)]
// pub struct AddCreditsRequest {
//     #[validate(range(
//         min = 1.0,
//         max = 5000.0,
//         message = "Amount must be between $1 and $5000"
//     ))]
//     pub amount: f64,
// }

// #[derive(Debug, Serialize)]
// pub struct AddCreditsResponse {
//     pub new_balance: f64,
//     pub added_amount: f64,
// }

// pub async fn add_credits_controller(
//     pool: web::Data<DbPool>,
//     req: HttpRequest,
//     body: web::Json<AddCreditsRequest>,
// ) -> Result<HttpResponse, AppError> {
//     body.validate()
//         .map_err(|e| AppError::ValidationError(e.to_string()))?;

//     let user_id = req
//         .extensions()
//         .get::<AuthMiddleware>()
//         .ok_or(AppError::InvalidCredentials)?
//         .user_id;

//     let amount = body.amount;

//     // Update user balance
//     let user = users::Entity::find_by_id(user_id)
//         .one(pool.as_ref())
//         .await
//         .map_err(|e| AppError::Database(e.to_string()))?
//         .ok_or(AppError::UserNotFound)?;

//     let new_balance = user.credit_balance + amount;
//     let mut user_active: users::ActiveModel = user.into();
//     user_active.credit_balance = Set(new_balance);
//     user_active
//         .update(pool.as_ref())
//         .await
//         .map_err(|e| AppError::Database(e.to_string()))?;

//     let response = AddCreditsResponse {
//         new_balance,
//         added_amount: amount,
//     };

//     Ok(HttpResponse::Ok().json(ApiResponse {
//         success: true,
//         message: format!("Successfully added ${:.2} to credit balance", amount),
//         data: Some(response),
//     }))
// }
