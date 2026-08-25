use crate::app::middlewares::AuthMiddleware;
use crate::app::models::billing;
use crate::app::{controllers::ApiResponse, db::DbPool};
use crate::app_errors::AppError;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PaymentConfirmationRequest {
    pub payment_id: String,
}

#[derive(Serialize)]
pub struct PaymentConfirmationResponse {
    pub confirmed: bool,
}

pub async fn payment_confirmation_controller(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    query: web::Query<PaymentConfirmationRequest>,
) -> Result<HttpResponse, AppError> {
    println!("{}", query.payment_id);
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let mut count = 0;
    let mut payment_confirmation = None;

    while count < 3 {
        payment_confirmation = billing::Entity::find()
            .filter(billing::Column::PaymentId.eq(&query.payment_id))
            .filter(billing::Column::UserId.eq(user_id))
            .one(pool.as_ref())
            .await?;

        if payment_confirmation.is_some() {
            break;
        }

        count += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    let resp_data = PaymentConfirmationResponse {
        confirmed: payment_confirmation.is_some(),
    };

    if payment_confirmation.is_none() {
        return Ok(HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "Payment not found".to_string(),
            data: Some(resp_data),
        }));
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Payment confirmed successfully".to_string(),
        data: Some(resp_data),
    }))
}
