// use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
// use std::sync::Arc;
// use crate::app::models::users;

// pub async fn deduct_usage_and_check_reload(
//     db: &DatabaseConnection,
//     dodo: Arc<dodopayments::Client>,
//     product_id: &str,
//     user_id: i32,
//     cost_dollars: f64,
// ) -> Result<f64, anyhow::Error> {
//     let txn = db.begin().await?;

//     let user = users::Entity::find_by_id(user_id)
//         .one(&txn)
//         .await?
//         .ok_or_else(|| anyhow::anyhow!("User {} not found", user_id))?;

//     let new_balance = user.credit_balance - cost_dollars;
//     let mut user_active: users::ActiveModel = user.clone().into();
//     user_active.credit_balance = Set(new_balance);

//     let should_reload = new_balance < 10.00
//         && !user.is_reloading
//         && user.dodo_customer_id.is_some();

//     let customer_id_opt = if should_reload {
//         user_active.is_reloading = Set(true);
//         user.dodo_customer_id.clone()
//     } else {
//         None
//     };

//     user_active.update(&txn).await?;
//     txn.commit().await?;

//     if let Some(dodo_customer_id) = customer_id_opt {
//         let dodo_clone = dodo.clone();
//         let product_id_owned = product_id.to_string();
//         let db_clone = db.clone();

//         actix_web::rt::spawn(async move {
//             let payload = serde_json::json!({
//                 "customer": {
//                     "customer_id": dodo_customer_id
//                 },
//                 "total_amount": 5000,
//                 "product_cart": [
//                     {
//                         "product_id": product_id_owned,
//                         "quantity": 1,
//                         "amount": 5000
//                     }
//                 ],
//                 "metadata": {
//                     "user_id": user_id.to_string(),
//                     "payment_type": "auto_reload"
//                 },
//                 "payment_link": false
//             });

//             let res = (*dodo_clone)
//                 .request(reqwest::Method::POST, "/payments")
//                 .json(&payload)
//                 .send()
//                 .await;

//             let is_success = match res {
//                 Ok(r) => r.status().is_success(),
//                 Err(_) => false,
//             };

//             if !is_success {
//                 eprintln!(
//                     "[Auto-Reload] Failed to dispatch off-session charge for user {}. Resetting is_reloading = false.",
//                     user_id
//                 );
//                 if let Ok(Some(u)) = users::Entity::find_by_id(user_id).one(&db_clone).await {
//                     let mut u_active: users::ActiveModel = u.into();
//                     u_active.is_reloading = Set(false);
//                     let _ = u_active.update(&db_clone).await;
//                 }
//             }
//         });
//     }

//     Ok(new_balance)
// }
