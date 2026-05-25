use serde::{Deserialize, Serialize};

pub mod auth;
pub mod github;
pub mod project;

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}
