use dodopayments::{Client, ClientConfig};

pub fn get_dodo_api_key() -> String {
    std::env::var("DODO_API_KEY").unwrap_or_else(|_| "test_dodo_api_key".to_string())
}

pub fn get_dodo_secret() -> String {
    std::env::var("DODO_SECRET").unwrap_or_else(|_| "whsec_test_secret".to_string())
}

pub fn get_dodo_client() -> Result<Client, dodopayments::Error> {
    let api_key = get_dodo_api_key();
    let config = ClientConfig::new("https://test.dodopayments.com").with_api_key(api_key);
    Client::new(config)
}
