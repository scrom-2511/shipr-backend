use crate::app::middlewares::AuthMiddleware;
use crate::app::models::users;
use crate::app::state::AppState;
use crate::app::{controllers::ApiResponse, db::DbPool};
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use dodopayments::models::{
    AttachExistingCustomer, CheckoutSessionsCreateParams, CustomerRequest, ProductItemReq,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DodoCheckoutRequest {
    pub amount: Option<f64>,
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DodoCheckoutResponse {
    pub checkout_url: String,
}

pub async fn dodo_checkout_controller(
    state: web::Data<AppState>,
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<DodoCheckoutRequest>,
) -> Result<HttpResponse, AppError> {
    println!("dodo_checkout_controller called");
    println!("body: {:?}", body);
    // 1. Get user if logged in
    let user_id = req
        .extensions()
        .get::<AuthMiddleware>()
        .map(|a| a.user_id)
        .unwrap();

    println!("user_id: {:?}", user_id);

    let user = users::Entity::find_by_id(user_id)
        .one(pool.as_ref())
        .await?;

    println!("user: {:?}", user);

    let user_email = user.clone().unwrap().email;
    let user_dodo_customer_id = user.clone().unwrap().dodo_customer_id.unwrap();

    println!("user_email: {:?}", user_email);

    let total_amount = body.amount.unwrap_or(25.0);
    let amount_cents = (total_amount * 100.0).round() as i64;

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    let return_target = body
        .return_url
        .as_ref()
        .cloned()
        .unwrap_or_else(|| format!("{}/checkout/success", frontend_url));

    let product_id = std::env::var("DODO_PRODUCT_ID").unwrap();
    println!("product_id: {:?}", product_id);

    use serde_json::json;
    use std::collections::HashMap;

    let mut metadata = HashMap::new();

    metadata.insert("user_id".to_string(), json!(user_id));
    metadata.insert("amount".to_string(), json!(amount_cents));

    // 3. Create checkout session using Dodo Payments SDK directly
    let result = state
        .client
        .checkout_sessions()
        .create()
        .body(CheckoutSessionsCreateParams {
            product_cart: Some(vec![ProductItemReq {
                product_id,
                quantity: 1,
                amount: Some(amount_cents),
                addons: None,
                credit_entitlements: None,
            }]),
            customer: Some(Box::new(CustomerRequest::AttachExistingCustomer(Box::new(
                AttachExistingCustomer {
                    customer_id: user_dodo_customer_id,
                },
            )))),
            metadata: Option::Some(Box::new(metadata)),
            return_url: Option::Some(return_target),
            show_saved_payment_methods: Option::Some(true),
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::DodoError(e.to_string()))?;

    println!("hi i am here after session creation");

    println!("{:?}", result);

    let checkout_url = result.checkout_url.unwrap();

    let resp_data = DodoCheckoutResponse {
        checkout_url: checkout_url.clone(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Dodo checkout created successfully".to_string(),
        data: Some(resp_data),
    }))
}
