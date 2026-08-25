use crate::app::middlewares::AuthMiddleware;
use crate::app::models::users;
use crate::app::state::AppState;
use crate::app::{controllers::ApiResponse, db::DbPool};
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};

use dodopayments::models::{
    AttachExistingCustomer, CheckoutSessionsCreateParams, CustomerRequest, OnDemandSubscription,
    ProductItemReq, SubscriptionData,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct DodoOndemandRequest {
    pub user_id: i32,
}

#[derive(Debug, Serialize)]
pub struct DodoOndemandResponse {
    pub checkout_url: String,
}

pub async fn dodo_ondemand_checkout_controller(
    state: web::Data<AppState>,
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<DodoOndemandRequest>,
) -> Result<HttpResponse, AppError> {
    println!("dodo_ondemand_checkout_controller called");

    let user_id = body.user_id;

    let user = users::Entity::find_by_id(user_id)
        .one(pool.as_ref())
        .await?;

    let user = user.ok_or(AppError::UserNotFound)?;

    let user_dodo_customer_id = user.dodo_customer_id.ok_or(AppError::UserNotFound)?;

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    let return_target = format!("{}/dashboard/billing?status=success", frontend_url);

    let product_id = std::env::var("DODO_ONDEMAND_PRODUCT_ID")
        .map_err(|_| AppError::DodoError("DODO_PRODUCT_ID is not configured".to_string()))?;

    let mut metadata = HashMap::new();

    metadata.insert("user_id".to_string(), json!(user_id));

    let result = state
        .client
        .checkout_sessions()
        .create()
        .body(CheckoutSessionsCreateParams {
            product_cart: Some(vec![ProductItemReq {
                product_id,
                quantity: 1,
                amount: None,
                addons: None,
                credit_entitlements: None,
            }]),

            customer: Some(Box::new(CustomerRequest::AttachExistingCustomer(Box::new(
                AttachExistingCustomer {
                    customer_id: user_dodo_customer_id,
                },
            )))),

            metadata: Some(Box::new(metadata)),

            return_url: Some(return_target),

            show_saved_payment_methods: Some(true),

            subscription_data: Some(Box::new(SubscriptionData {
                on_demand: Some(Box::new(OnDemandSubscription {
                    mandate_only: true,
                    adaptive_currency_fees_inclusive: None,
                    product_currency: None,
                    product_description: None,
                    product_price: None,
                })),
                trial_period_days: None,
            })),

            ..Default::default()
        })
        .await
        .map_err(|e| AppError::DodoError(e.to_string()))?;

    println!("Dodo checkout created");
    println!("{:?}", result);

    let checkout_url = result
        .checkout_url
        .ok_or_else(|| AppError::DodoError("Dodo did not return checkout URL".to_string()))?;

    let resp_data = DodoOndemandResponse { checkout_url };

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Dodo checkout created successfully".to_string(),
        data: Some(resp_data),
    }))
}
