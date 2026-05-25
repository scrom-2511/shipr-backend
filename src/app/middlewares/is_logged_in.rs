use actix_web::{
    Error, HttpMessage,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};

use crate::{
    app::{controllers::auth::decode_token, middlewares::AuthMiddleware},
    app_errors::AppError,
};

pub async fn is_logged_in(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let auth_token = req.cookie("token");

    if auth_token.is_some() {
        println!(
            "auth_token is some, {}",
            auth_token.as_ref().unwrap().value()
        );
        let decoded = decode_token(auth_token.unwrap().value());

        match decoded {
            Ok(claims) => {
                req.extensions_mut().insert(AuthMiddleware {
                    user_id: claims.user_id,
                });
            }
            Err(_) => {
                return Err(Error::from(AppError::InvalidCredentials));
            }
        }
    } else {
        return Err(Error::from(AppError::InvalidCredentials));
    }

    next.call(req).await
}
