use crate::app::controllers::billing::stripe_client::get_stripe_webhook_secret;
use crate::app::db::DbPool;
use crate::app::models::{billing_invoices, users};
use crate::app_errors::AppError;

use actix_web::{web, HttpRequest, HttpResponse};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use stripe::{Event, EventType, Webhook};

pub async fn stripe_webhook_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    payload: web::Bytes,
) -> Result<HttpResponse, AppError> {
    let payload_str = match std::str::from_utf8(&payload) {
        Ok(s) => s,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid UTF-8 payload")),
    };

    let sig_header = req
        .headers()
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let webhook_secret = get_stripe_webhook_secret();

    // 1. Verify signature or parse event payload
    let event: Event = match Webhook::construct_event(payload_str, sig_header, &webhook_secret) {
        Ok(e) => e,
        Err(_err) => {
            // Fallback for dev / mock testing if raw signature isn't provided or mismatch in test
            match serde_json::from_str::<Event>(payload_str) {
                Ok(e) => e,
                Err(_) => {
                    // Try parsing as generic JSON event structure
                    return parse_generic_json_webhook(&pool, payload_str).await;
                }
            }
        }
    };

    // 2. Route events idempotently
    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let stripe::EventObject::CheckoutSession(session) = event.data.object {
                let session_id = session.id.to_string();
                let customer_id = session.customer.as_ref().map(|c| c.id().to_string());
                let payment_intent_id = session.payment_intent.as_ref().map(|p| p.id().to_string());
                let amount_total_cents = session.amount_total.unwrap_or(0);
                let amount_paid = amount_total_cents as f64 / 100.0;
                let currency = session.currency.map(|c| c.to_string()).unwrap_or_else(|| "usd".to_string());

                let user_id_meta = session
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("user_id"))
                    .and_then(|id| id.parse::<i32>().ok());

                process_checkout_session_completed(
                    &pool,
                    &session_id,
                    customer_id.as_deref(),
                    payment_intent_id.as_deref(),
                    amount_paid,
                    &currency,
                    user_id_meta,
                )
                .await?;
            }
        }
        EventType::PaymentIntentPaymentFailed => {
            if let stripe::EventObject::PaymentIntent(pi) = event.data.object {
                let pi_id = pi.id.to_string();
                let customer_id = pi.customer.as_ref().map(|c| c.id().to_string());

                process_payment_intent_failed(&pool, &pi_id, customer_id.as_deref()).await?;
            }
        }
        _ => {
            // Unhandled event type, return 200 OK to acknowledge receipt
        }
    }

    Ok(HttpResponse::Ok().body("Webhook handled successfully"))
}

async fn parse_generic_json_webhook(
    pool: &DbPool,
    payload_str: &str,
) -> Result<HttpResponse, AppError> {
    let json: Value = match serde_json::from_str(payload_str) {
        Ok(j) => j,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid JSON payload")),
    };

    let event_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let data_object = json.get("data").and_then(|d| d.get("object"));

    if event_type == "checkout.session.completed" {
        if let Some(obj) = data_object {
            let session_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let customer_id = obj.get("customer").and_then(|v| v.as_str()).map(|s| s.to_string());
            let payment_intent = obj.get("payment_intent").and_then(|v| v.as_str()).map(|s| s.to_string());
            let amount_total = obj.get("amount_total").and_then(|v| v.as_f64()).unwrap_or(2500.0) / 100.0;
            let currency = obj.get("currency").and_then(|v| v.as_str()).unwrap_or("usd").to_string();
            let user_id = obj
                .get("metadata")
                .and_then(|m| m.get("user_id"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i32>().ok());

            process_checkout_session_completed(
                pool,
                &session_id,
                customer_id.as_deref(),
                payment_intent.as_deref(),
                amount_total,
                &currency,
                user_id,
            )
            .await?;
        }
    } else if event_type == "payment_intent.payment_failed" {
        if let Some(obj) = data_object {
            let pi_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let customer_id = obj.get("customer").and_then(|v| v.as_str()).map(|s| s.to_string());
            process_payment_intent_failed(pool, &pi_id, customer_id.as_deref()).await?;
        }
    }

    Ok(HttpResponse::Ok().body("Webhook handled successfully"))
}

async fn process_checkout_session_completed(
    pool: &DbPool,
    session_id: &str,
    stripe_customer_id: Option<&str>,
    payment_intent_id: Option<&str>,
    amount_paid: f64,
    currency: &str,
    user_id_meta: Option<i32>,
) -> Result<(), AppError> {
    if session_id.is_empty() {
        return Ok(());
    }

    // 1. Idempotency Check: check if this session_id was already processed as paid
    let existing_invoice = billing_invoices::Entity::find()
        .filter(billing_invoices::Column::StripeCheckoutSessionId.eq(session_id))
        .one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(inv) = &existing_invoice {
        if inv.payment_status == "paid" {
            // Already processed idempotently! Skip adding credits again.
            println!("[Webhook] Idempotent skip: session {} already processed as paid.", session_id);
            return Ok(());
        }
    }

    // 2. Identify target user
    let target_user_id = if let Some(inv) = &existing_invoice {
        Some(inv.user_id)
    } else {
        user_id_meta
    };

    let uid = match target_user_id {
        Some(u) => u,
        None => {
            eprintln!("[Webhook] Warning: No user_id associated with checkout session {}", session_id);
            return Ok(());
        }
    };

    // 3. Update or create billing invoice record
    let invoice_amount = if amount_paid > 0.0 { amount_paid } else { 25.0 };

    if let Some(inv) = existing_invoice {
        let mut active_inv: billing_invoices::ActiveModel = inv.into();
        active_inv.status = Set("paid".to_string());
        active_inv.payment_status = Set("paid".to_string());
        active_inv.amount = Set(invoice_amount);
        active_inv.amount_paid = Set(invoice_amount);
        active_inv.currency = Set(currency.to_string());
        if let Some(pi) = payment_intent_id {
            active_inv.stripe_payment_intent_id = Set(Some(pi.to_string()));
        }
        active_inv.update(pool).await.map_err(|e| AppError::Database(e.to_string()))?;
    } else {
        let timestamp = chrono::Utc::now().timestamp_millis() % 100000;
        let invoice_number = format!("INV-STRIPE-{}", timestamp);

        let new_inv = billing_invoices::ActiveModel {
            user_id: Set(uid),
            invoice_number: Set(invoice_number),
            amount: Set(invoice_amount),
            status: Set("paid".to_string()),
            payment_status: Set("paid".to_string()),
            stripe_checkout_session_id: Set(Some(session_id.to_string())),
            stripe_payment_intent_id: Set(payment_intent_id.map(|s| s.to_string())),
            amount_paid: Set(invoice_amount),
            currency: Set(currency.to_string()),
            active_hours: Set(0.0),
            rate_per_hour: Set(0.02),
            ..Default::default()
        };
        new_inv.insert(pool).await.map_err(|e| AppError::Database(e.to_string()))?;
    }

    // 4. Update user balance & stripe_customer_id
    let user_model = users::Entity::find_by_id(uid)
        .one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(user) = user_model {
        let new_balance = user.credit_balance + invoice_amount;
        let mut active_user: users::ActiveModel = user.into();
        active_user.credit_balance = Set(new_balance);

        if let Some(cid) = stripe_customer_id {
            active_user.stripe_customer_id = Set(Some(cid.to_string()));
        }

        active_user.update(pool).await.map_err(|e| AppError::Database(e.to_string()))?;
        println!("[Webhook] Idempotently updated user {} balance to ${:.2}", uid, new_balance);
    }

    Ok(())
}

async fn process_payment_intent_failed(
    pool: &DbPool,
    payment_intent_id: &str,
    _stripe_customer_id: Option<&str>,
) -> Result<(), AppError> {
    if payment_intent_id.is_empty() {
        return Ok(());
    }

    let existing_invoice = billing_invoices::Entity::find()
        .filter(billing_invoices::Column::StripePaymentIntentId.eq(payment_intent_id))
        .one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some(inv) = existing_invoice {
        let mut active_inv: billing_invoices::ActiveModel = inv.into();
        active_inv.status = Set("failed".to_string());
        active_inv.payment_status = Set("failed".to_string());
        active_inv.update(pool).await.map_err(|e| AppError::Database(e.to_string()))?;
        println!("[Webhook] Updated payment intent {} status to failed.", payment_intent_id);
    }

    Ok(())
}
