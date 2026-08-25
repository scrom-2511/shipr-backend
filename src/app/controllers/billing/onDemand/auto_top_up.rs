use std::collections::HashMap;

use actix_web::web;
use dodopayments::models::{Currency, SubscriptionsChargeParams};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::json;

use crate::app::db::DbPool;
use crate::app::models::users;
use crate::app::state::AppState;
use crate::app_errors::AppError;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AutoTopUpRequest {
    user_id: i32,
}

pub async fn auto_top_up(
    state: web::Data<AppState>,
    pool: DbPool,
    user_id: i32,
) -> Result<(), AppError> {
    let user = users::Entity::find_by_id(user_id)
        .one(&pool)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let subscription_id = user.dodo_subscription_id.ok_or(AppError::UserNotFound)?;

    let mut metadata = HashMap::new();

    metadata.insert("user_id".to_string(), json!(user_id));
    metadata.insert("payment_type".to_string(), json!("auto_top_up"));

    let result = state
        .client
        .subscriptions()
        .charge()
        .subscription_id(subscription_id)
        .body(SubscriptionsChargeParams {
            product_price: Some(5000),
            product_currency: Some(Box::new(Currency::Usd)),
            metadata: Some(Box::new(metadata)),
            ..Default::default()
        })
        .send()
        .await
        .map_err(|e| AppError::DodoError(e.to_string()))?;

    println!("Auto top up result: {:#?}", result);

    Ok(())
}
