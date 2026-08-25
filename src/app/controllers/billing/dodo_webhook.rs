use crate::app::db::DbPool;
use crate::app::state::AppState;
use crate::app::{models::billing, models::users};
use crate::app_errors::AppError;
use crate::core::controller::storage::redis::Redis;

use actix_web::{HttpRequest, HttpResponse, web};
use base64::{Engine, engine::general_purpose::STANDARD};
use dodopayments::models::{Currency, Payment};
use dodopayments::{Client, models::PaymentSucceededWebhookEvent};
use hmac::{Hmac, KeyInit, Mac};
use redis::AsyncCommands;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::Value;
use sha2::Sha256;
use subtle::ConstantTimeEq;

pub async fn dodo_webhook_controller(
    state: web::Data<AppState>,
    body: web::Bytes,
    req: HttpRequest,
    pool: web::Data<DbPool>,
    redis: web::Data<Redis>,
) -> Result<HttpResponse, AppError> {
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("Invalid JSON payload".to_string()))?;

    let event_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let data = payload.get("data");

    println!("event_type : {}", event_type);
    println!("FULL WEBHOOK: {}", payload);
    // println!("data : {}", data);

    match event_type {
        "payment.succeeded" => {
            if let Some(data_obj) = data {
                let data = serde_json::from_value::<Payment>(data_obj.to_owned()).unwrap();

                let existing_txn = billing::Entity::find()
                    .filter(billing::Column::PaymentId.eq(data.payment_id.clone()))
                    .one(pool.as_ref())
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

                if existing_txn.is_some() {
                    return Ok(HttpResponse::Ok().finish());
                }

                let is_valid = verify_webhook(&req, &body)?;

                if !is_valid {
                    return Ok(HttpResponse::Unauthorized().finish());
                }
                process_payment_succeeded(&state.db, data, redis).await?;
            }
        }
        "subscription.active" => {
            if let Some(data_obj) = data {
                let subscription_id = data_obj
                    .get("subscription_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::BadRequest("Missing subscription_id".to_string()))?;

                let user_id = data_obj
                    .get("metadata")
                    .and_then(|m| m.get("user_id"))
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        AppError::BadRequest("Missing user_id in metadata".to_string())
                    })? as i32;

                let user = users::Entity::find_by_id(user_id)
                    .one(pool.as_ref())
                    .await?
                    .ok_or(AppError::UserNotFound)?;

                let mut active_user: users::ActiveModel = user.into();

                active_user.dodo_subscription_id = Set(Some(subscription_id.to_string()));
                active_user.auto_topup_enabled = Set(true);

                active_user.update(pool.as_ref()).await?;

                println!(
                    "Saved subscription {} for user {}",
                    subscription_id, user_id
                );
            }
        }

        _ => {}
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "received": true,
        "status": "processed",
        "event_type": event_type
    })))
}

fn verify_webhook(req: &HttpRequest, body: &[u8]) -> Result<bool, AppError> {
    let webhook_id = req
        .headers()
        .get("webhook-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing or invalid webhook-id header".to_string()))?;

    let webhook_timestamp = req
        .headers()
        .get("webhook-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::BadRequest("Missing or invalid webhook-timestamp header".to_string())
        })?;

    let webhook_signature = req
        .headers()
        .get("webhook-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::BadRequest("Missing or invalid webhook-signature header".to_string())
        })?;

    let mut signed_payload = Vec::new();

    signed_payload.extend_from_slice(webhook_id.as_bytes());
    signed_payload.extend_from_slice(b".");
    signed_payload.extend_from_slice(webhook_timestamp.as_bytes());
    signed_payload.extend_from_slice(b".");
    signed_payload.extend_from_slice(body);

    let raw_secret =
        std::env::var("DODO_PAYMENTS_WEBHOOK_KEY").map_err(|_| AppError::InternalServerError)?;

    let secret_str = raw_secret.strip_prefix("whsec_").unwrap_or(&raw_secret);
    let secret_bytes = STANDARD
        .decode(secret_str)
        .map_err(|_| AppError::InternalServerError)?;

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret_bytes.as_slice()) {
        Ok(mac) => mac,
        Err(_) => return Ok(false),
    };

    mac.update(&signed_payload);

    let expected_signature = mac.finalize().into_bytes();

    let result = webhook_signature
        .split_whitespace()
        .filter_map(|signature| signature.strip_prefix("v1,"))
        .any(|signature| {
            let provided_signature = match STANDARD.decode(signature) {
                Ok(value) => value,
                Err(_) => return false,
            };

            provided_signature
                .as_slice()
                .ct_eq(expected_signature.as_slice())
                .into()
        });

    Ok(result)
}

async fn process_payment_succeeded(
    db: &DatabaseConnection,
    data: Payment,
    redis: web::Data<Redis>,
) -> Result<(), AppError> {
    if data.total_amount == 0 {
        println!("Mandate-only payment, not adding credits");
        return Ok(());
    }
    let user_id = data
        .metadata
        .get("user_id")
        .and_then(|v| {
            v.as_i64()
                .map(|n| n as i32)
                .or_else(|| v.as_str().and_then(|s| s.parse::<i32>().ok()))
        })
        .ok_or_else(|| {
            AppError::BadRequest("Missing or invalid user_id in metadata".to_string())
        })?;

    let amount = data
        .metadata
        .get("amount")
        .and_then(|v| {
            v.as_i64()
                .map(|n| n as i64)
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .ok_or_else(|| AppError::BadRequest("Missing or invalid amount in metadata".to_string()))?;

    println!("user_id : {}", user_id);

    let currency =
        serde_json::to_string(&data.currency).map_err(|_| AppError::InternalServerError)?;

    println!("currency : {}", currency);

    let txn = db
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    println!("txn : {:#?}", txn);
    println!("payment_amount : {:#?}", data.total_amount);
    println!("payment_currency : {:#?}", data.currency);
    println!("payment_id : {:#?}", data.payment_id);
    println!("session_id : {:#?}", data.checkout_session_id);
    println!("payment_method_id : {:#?}", data.payment_method_id);

    let add_txn = billing::ActiveModel {
        user_id: Set(user_id),
        amount: Set(amount),
        currency: Set(currency),
        payment_id: Set(data.payment_id),
        ..Default::default()
    };

    println!("add_txn : {:#?}", add_txn);

    add_txn
        .insert(&txn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    println!("added billing txn");

    let user = users::Entity::find_by_id(user_id)
        .one(&txn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("User not found".to_string()))?;

    let new_balance = user.credit_balance + amount;

    let mut active_user: users::ActiveModel = user.into();

    active_user.credit_balance = Set(new_balance);

    let dodo_customer_id = data.customer.customer_id;
    active_user.dodo_customer_id = Set(Some(dodo_customer_id));

    active_user
        .update(&txn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    txn.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    println!("committed transaction");

    let mut conn = redis.get_conn();
    let _: () = conn
        .set(format!("credits:{}", user_id), new_balance)
        .await?;

    Ok(())
}
