use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::{ProjectStatus, projects, users};
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProjectBillingUsage {
    pub id: i32,
    pub project_id: String,
    pub full_name: String,
    pub status: ProjectStatus,
    pub active_seconds: i64,
    pub active_hours: f64,
    pub hourly_rate: f64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceItem {
    pub id: i32,
    pub invoice_number: String,
    pub amount: f64,
    pub status: String,
    pub active_hours: f64,
    pub rate_per_hour: f64,
    pub period_start: Option<NaiveDateTime>,
    pub period_end: Option<NaiveDateTime>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethodItem {
    pub id: i32,
    pub card_brand: String,
    pub last4: String,
    pub exp_month: i32,
    pub exp_year: i32,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct BillingDetailsResponse {
    pub hourly_rate: f64,
    pub credit_balance: i64,
    pub total_active_seconds: i64,
    pub total_active_hours: f64,
    pub current_month_cost: f64,
    pub estimated_monthly_cost: f64,
    pub projects: Vec<ProjectBillingUsage>,
}

const HOURLY_RATE: f64 = 0.02; // $0.02 per hour per microVM

pub async fn get_billing_details_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .ok_or(AppError::InvalidCredentials)?
        .user_id;

    // Fetch user info
    let user_info = users::Entity::find_by_id(user_id)
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let credit_balance = user_info.as_ref().map(|u| u.credit_balance).unwrap_or(5000);

    // Fetch project usage
    let project_rows = projects::Entity::find()
        .filter(projects::Column::UserId.eq(user_id))
        .order_by_desc(projects::Column::CreatedAt)
        .all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut total_active_seconds: i64 = 0;
    let mut projects: Vec<ProjectBillingUsage> = Vec::new();

    for p in project_rows {
        let secs = p.active_seconds;
        total_active_seconds += secs;
        let hours = secs as f64 / 3600.0;
        let cost = (hours * HOURLY_RATE * 10000.0).round() / 10000.0;

        projects.push(ProjectBillingUsage {
            id: p.id,
            project_id: p.project_id,
            full_name: p.full_name,
            status: p.status,
            active_seconds: secs,
            active_hours: (hours * 100.0).round() / 100.0,
            hourly_rate: HOURLY_RATE,
            cost,
        });
    }

    let total_active_hours = (total_active_seconds as f64 / 3600.0 * 100.0).round() / 100.0;
    let current_month_cost = (total_active_hours * HOURLY_RATE * 100.0).round() / 100.0;
    let estimated_monthly_cost = if projects.is_empty() {
        0.0
    } else {
        ((projects.len() as f64) * 24.0 * 30.0 * HOURLY_RATE * 100.0).round() / 100.0
    };

    let response = BillingDetailsResponse {
        hourly_rate: HOURLY_RATE,
        credit_balance,
        total_active_seconds,
        total_active_hours,
        current_month_cost,
        estimated_monthly_cost,
        projects,
    };

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Billing details retrieved successfully".to_string(),
        data: Some(response),
    }))
}
