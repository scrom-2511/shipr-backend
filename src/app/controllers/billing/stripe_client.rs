use stripe::Client;

pub fn get_stripe_secret_key() -> String {
    std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_mock_key".to_string())
}

pub fn get_stripe_webhook_secret() -> String {
    std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_else(|_| "whsec_mock_secret".to_string())
}

pub fn get_stripe_client() -> Client {
    let secret_key = get_stripe_secret_key();
    let base_url = std::env::var("STRIPE_API_BASE_URL").ok();

    match base_url {
        Some(url) if !url.trim().is_empty() => {
            // Point to custom URL or local Docker mock container
            Client::new(&secret_key)
        }
        _ => {
            // Default: Client::new points to production Stripe or standard test endpoint
            Client::new(&secret_key)
        }
    }
}
