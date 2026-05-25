pub mod github_signup;
pub mod signin;
pub mod signup;

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::app_errors::AppError;

pub const JWT_SECRET: &[u8] = b"shipr_jwt_secret_key_2026";

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub iat: u64,
    pub exp: u64,
}

pub fn generate_token(user_id: i32) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        user_id,
        iat: now,
        exp: now + (24 * 60 * 60),
    };

    let header = Header::new(Algorithm::HS256);
    let token = encode(&header, &claims, &EncodingKey::from_secret(JWT_SECRET))?;
    Ok(token)
}

pub fn decode_token(token: &str) -> Result<Claims, AppError> {
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(JWT_SECRET);
    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    let decoded = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation);

    match decoded {
        Ok(token_data) => Ok(token_data.claims),
        Err(_) => Err(AppError::InvalidCredentials),
    }
}
