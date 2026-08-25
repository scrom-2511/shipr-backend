// use reqwest::Method;
// use serde_json::Value;
// use std::collections::HashMap;

// pub struct CustomerNew {
//     pub email: String,
// }

// pub enum CustomerRequest {
//     New(CustomerNew),
//     Id(String),
// }

// pub struct CheckoutSessionParams {
//     pub product_id: String,
//     pub amount_cents: i64,
//     pub customer: CustomerRequest,
//     pub metadata: HashMap<String, String>,
//     pub return_url: Option<String>,
// }

// pub struct DodoCheckoutSession {
//     pub payment_link: Option<String>,
// }

// use dodopayments::models::{
//     CheckoutSessionsCreateParams, CustomerRequest as DodoCustomerRequest, NewCustomer, ProductItemReq,
// };

// pub async fn create_checkout_session(
//     client: &dodopayments::Client,
//     params: CheckoutSessionParams,
// ) -> Result<DodoCheckoutSession, anyhow::Error> {
//     let email = match params.customer {
//         CustomerRequest::New(new_cust) => new_cust.email,
//         CustomerRequest::Id(id) => id,
//     };

//     let frontend_url = std::env::var("FRONTEND_URL")
//         .unwrap_or_else(|_| "http://localhost:5173".to_string());

//     let return_url = params
//         .return_url
//         .unwrap_or_else(|| format!("{}/dashboard/billing?status=success", frontend_url));

//     let session = client
//         .checkout_sessions()
//         .create()
//         .body(CheckoutSessionsCreateParams {
//             product_cart: Some(vec![ProductItemReq {
//                 product_id: params.product_id,
//                 quantity: 1,
//                 amount: Some(params.amount_cents),
//                 addons: None,
//                 credit_entitlements: None,
//             }]),
//             customer: Some(DodoCustomerRequest::New(NewCustomer {
//                 email,
//                 name: None,
//                 phone_number: None,
//             })),
//             metadata: Some(params.metadata),
//             return_url: Some(return_url),
//             ..Default::default()
//         })
//         .await?;

//     let payment_link = session.checkout_url.or(session.payment_link);

//     Ok(DodoCheckoutSession { payment_link })
// }

// pub trait DodoClientExt {
//     fn request(&self, method: Method, path: &str) -> DodoRequestBuilder;
// }

// pub struct DodoRequestBuilder {
//     method: Method,
//     path: String,
//     json_body: Option<Value>,
// }

// impl DodoRequestBuilder {
//     pub fn new(method: Method, path: &str) -> Self {
//         Self {
//             method,
//             path: path.to_string(),
//             json_body: None,
//         }
//     }

//     pub fn json(mut self, body: &Value) -> Self {
//         self.json_body = Some(body.clone());
//         self
//     }

//     pub async fn send(self) -> Result<reqwest::Response, anyhow::Error> {
//         let api_key = std::env::var("DODO_PAYMENTS_API_KEY")
//             .or_else(|_| std::env::var("DODO_API_KEY"))
//             .unwrap_or_else(|_| "test_dodo_api_key".to_string());

//         let base_url = std::env::var("DODO_API_URL")
//             .unwrap_or_else(|_| "https://test.dodopayments.com".to_string());

//         let url = if self.path.starts_with('/') {
//             format!("{}{}", base_url, self.path)
//         } else {
//             format!("{}/{}", base_url, self.path)
//         };

//         let http_client = reqwest::Client::new();
//         let mut req = http_client
//             .request(self.method, &url)
//             .header("Authorization", format!("Bearer {}", api_key))
//             .header("Content-Type", "application/json");

//         if let Some(body) = self.json_body {
//             req = req.json(&body);
//         }

//         let res = req.send().await?;
//         Ok(res)
//     }
// }

// impl DodoClientExt for dodopayments::Client {
//     fn request(&self, method: Method, path: &str) -> DodoRequestBuilder {
//         DodoRequestBuilder::new(method, path)
//     }
// }
