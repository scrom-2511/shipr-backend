use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub client: Arc<dodopayments::Client>,
    pub product_id: String,
}
