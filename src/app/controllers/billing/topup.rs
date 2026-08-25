// use actix_web::{web, HttpResponse};
// use dodopayments::models::{
//     CheckoutSessionsCreateParams, CustomerRequest, NewCustomer, ProductItemReq,
// };
// use sea_orm::EntityTrait;
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;

// use crate::app::models::users;
// use crate::app::state::AppState;
// use crate::app_errors::AppError;

// #[derive(Debug, Deserialize)]
// pub struct TopupRequest {
//     pub user_id: i32,
//     pub amount_dollars: f64,
// }

// #[derive(Debug, Serialize)]
// pub struct TopupResponse {
//     pub checkout_url: Option<String>,
//     pub payment_link: Option<String>,
// }

// pub async fn topup_handler(
//     state: web::Data<AppState>,
//     payload: web::Json<TopupRequest>,
// ) -> Result<HttpResponse, AppError> {
//     let user = users::Entity::find_by_id(payload.user_id)
//         .one(&state.db)
//         .await
//         .map_err(|e| AppError::Database(e.to_string()))?
//         .ok_or(AppError::UserNotFound)?;

//     let amount_cents = (payload.amount_dollars * 100.0).round() as i64;

//     let mut metadata = HashMap::new();
//     metadata.insert("user_id".to_string(), serde_json::Value::String(payload.user_id.to_string()));
//     metadata.insert("payment_type".to_string(), serde_json::Value::String("manual_topup".to_string()));

//     let frontend_url = std::env::var("FRONTEND_URL")
//         .unwrap_or_else(|_| "http://localhost:5173".to_string());
//     let return_url = format!("{}/dashboard/billing?status=success", frontend_url);

//     let session = state
//         .dodo
//         .checkout_sessions()
//         .create()
//         .body(CheckoutSessionsCreateParams {
//             product_cart: Some(vec![ProductItemReq {
//                 product_id: state.product_id.clone(),
//                 quantity: 1,
//                 amount: Some(amount_cents),
//                 addons: None,
//                 credit_entitlements: None,
//             }]),
//             customer: Some(CustomerRequest {
//                 email: user.email.clone(),
//                 name: None,
//                 phone_number: None,
//             }),
//             metadata: Some(Box::new(metadata)),
//             return_url: Some(return_url),
//             ..Default::default()
//         })
//         .await
//         .map_err(|e| AppError::BadRequest(format!("Dodo checkout_sessions failed: {}", e)))?;

//     let checkout_url = session
//         .checkout_url
//         .ok_or_else(|| AppError::BadRequest("Missing checkout URL in Dodo response".to_string()))?;

//     Ok(HttpResponse::Ok().json(TopupResponse {
//         checkout_url: Some(checkout_url.clone()),
//         payment_link: Some(checkout_url),
//     }))
// }
