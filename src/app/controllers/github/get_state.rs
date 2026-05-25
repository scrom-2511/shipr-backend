use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::{
    app::{
        controllers::{ApiResponse, auth::generate_token},
        middlewares::AuthMiddleware,
    },
    app_errors::AppError,
};

#[derive(Debug, Serialize, Deserialize)]
struct StateResponse {
    state: String,
}

pub async fn get_state(req: HttpRequest) -> Result<HttpResponse, AppError> {
    println!("get_state called");
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;
    println!("get_state {}", user_id);

    let state = generate_token(user_id)?;

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "State generated successfully".to_string(),
        data: Some(StateResponse { state }),
    }))
}
