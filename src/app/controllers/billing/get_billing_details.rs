use crate::app::controllers::ApiResponse;
use crate::app::db::DbPool;
use crate::app::middlewares::AuthMiddleware;
use crate::app::models::{billing_invoices, payment_methods, projects, users};
use crate::app_errors::AppError;

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProjectBillingUsage {
    pub id: i32,
    pub project_id: String,
    pub full_name: String,
    pub status: String,
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
    pub plan_name: String,
    pub hourly_rate: f64,
    pub credit_balance: f64,
    pub total_active_seconds: i64,
    pub total_active_hours: f64,
    pub current_month_cost: f64,
    pub estimated_monthly_cost: f64,
    pub projects: Vec<ProjectBillingUsage>,
    pub invoices: Vec<InvoiceItem>,
    pub payment_method: Option<PaymentMethodItem>,
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

    let credit_balance = user_info.as_ref().map(|u| u.credit_balance).unwrap_or(50.0);
    let plan_name = user_info
        .as_ref()
        .map(|u| u.plan_tier.clone())
        .unwrap_or_else(|| "Developer".to_string());

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

    // Ensure payment method exists
    let mut payment_method_model = payment_methods::Entity::find()
        .filter(payment_methods::Column::UserId.eq(user_id))
        .filter(payment_methods::Column::IsDefault.eq(true))
        .one(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if payment_method_model.is_none() {
        let new_pm = payment_methods::ActiveModel {
            user_id: Set(user_id),
            card_brand: Set("Visa".to_string()),
            last4: Set("4242".to_string()),
            exp_month: Set(12),
            exp_year: Set(2028),
            is_default: Set(true),
            ..Default::default()
        };
        let _ = new_pm.insert(pool.as_ref()).await;

        payment_method_model = payment_methods::Entity::find()
            .filter(payment_methods::Column::UserId.eq(user_id))
            .filter(payment_methods::Column::IsDefault.eq(true))
            .one(pool.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    }

    let payment_method = payment_method_model.map(|pm| PaymentMethodItem {
        id: pm.id,
        card_brand: pm.card_brand,
        last4: pm.last4,
        exp_month: pm.exp_month,
        exp_year: pm.exp_year,
        is_default: pm.is_default,
    });

    // Fetch invoices
    let invoice_models = billing_invoices::Entity::find()
        .filter(billing_invoices::Column::UserId.eq(user_id))
        .order_by_desc(billing_invoices::Column::CreatedAt)
        .all(pool.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut invoices: Vec<InvoiceItem> = invoice_models
        .into_iter()
        .map(|inv| InvoiceItem {
            id: inv.id,
            invoice_number: inv.invoice_number,
            amount: inv.amount,
            status: inv.status,
            active_hours: inv.active_hours,
            rate_per_hour: inv.rate_per_hour,
            period_start: inv.period_start,
            period_end: inv.period_end,
            created_at: inv.created_at,
        })
        .collect();

    // Seed initial invoice if empty
    if invoices.is_empty() {
        let inv_num = format!("INV-2026-{:04}", user_id * 10 + 1);
        let initial_amount = current_month_cost;
        let new_inv = billing_invoices::ActiveModel {
            user_id: Set(user_id),
            invoice_number: Set(inv_num),
            amount: Set(initial_amount),
            status: Set("paid".to_string()),
            active_hours: Set(total_active_hours),
            rate_per_hour: Set(HOURLY_RATE),
            ..Default::default()
        };
        let _ = new_inv.insert(pool.as_ref()).await;

        let invoice_models = billing_invoices::Entity::find()
            .filter(billing_invoices::Column::UserId.eq(user_id))
            .order_by_desc(billing_invoices::Column::CreatedAt)
            .all(pool.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        invoices = invoice_models
            .into_iter()
            .map(|inv| InvoiceItem {
                id: inv.id,
                invoice_number: inv.invoice_number,
                amount: inv.amount,
                status: inv.status,
                active_hours: inv.active_hours,
                rate_per_hour: inv.rate_per_hour,
                period_start: inv.period_start,
                period_end: inv.period_end,
                created_at: inv.created_at,
            })
            .collect();
    }

    let response = BillingDetailsResponse {
        plan_name,
        hourly_rate: HOURLY_RATE,
        credit_balance,
        total_active_seconds,
        total_active_hours,
        current_month_cost,
        estimated_monthly_cost,
        projects,
        invoices,
        payment_method,
    };

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Billing details retrieved successfully".to_string(),
        data: Some(response),
    }))
}
