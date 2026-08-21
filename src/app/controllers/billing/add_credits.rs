use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct AddCreditsRequest {
    #[validate(range(min = 1.0, max = 5000.0, message = "Amount must be between $1 and $5000"))]
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct AddCreditsResponse {
    pub new_balance: f64,
    pub added_amount: f64,
    pub invoice_number: String,
}

pub async fn add_credits_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<AddCreditsRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or(AppError::InvalidCredentials)?
        .user_id;

    let amount = body.amount;

    // Update user balance
    let updated_user: (f64,) = sqlx::query_as(
        "UPDATE users SET credit_balance = COALESCE(credit_balance, 0.0) + $1 WHERE id = $2 RETURNING credit_balance",
    )
    .bind(amount)
    .bind(user_id)
    .fetch_one(pool.as_ref())
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let new_balance = updated_user.0;

    // Create an invoice record for credit addition
    let timestamp = chrono::Utc::now().timestamp_millis() % 100000;
    let invoice_number = format!("INV-TOPUP-{}", timestamp);

    let _ = sqlx::query(
        "INSERT INTO billing_invoices (user_id, invoice_number, amount, status, active_hours, rate_per_hour) VALUES ($1, $2, $3, 'paid', 0.0, 0.02)",
    )
    .bind(user_id)
    .bind(&invoice_number)
    .bind(amount)
    .execute(pool.as_ref())
    .await;

    let response = AddCreditsResponse {
        new_balance,
        added_amount: amount,
        invoice_number,
    };

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("Successfully added ${:.2} to credit balance", amount),
        data: Some(response),
    }))
}
