use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::{billing_invoices, users};
use crate::app_errors::AppError;

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
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
    body.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or(AppError::InvalidCredentials)?
        .user_id;

    let amount = body.amount;

    // Update user balance
    let user = users::Entity::find_by_id(user_id)
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or(AppError::UserNotFound)?;

    let new_balance = user.credit_balance + amount;
    let mut user_active: users::ActiveModel = user.into();
    user_active.credit_balance = Set(new_balance);
    user_active
        .update(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Create an invoice record for credit addition
    let timestamp = chrono::Utc::now().timestamp_millis() % 100000;
    let invoice_number = format!("INV-TOPUP-{}", timestamp);

    let new_invoice = billing_invoices::ActiveModel {
        user_id: Set(user_id),
        invoice_number: Set(invoice_number.clone()),
        amount: Set(amount),
        status: Set("paid".to_string()),
        active_hours: Set(0.0),
        rate_per_hour: Set(0.02),
        ..Default::default()
    };

    let _ = new_invoice.insert(pool.as_ref()).await;

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
