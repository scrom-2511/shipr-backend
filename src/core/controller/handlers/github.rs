// use actix_web::{
//     HttpResponse,
//     web::{self},
// };

// use crate::{
//     core::app_types::{EventType, InstallationStore},
//     core::controller::queue::redeploy_queue::ReDeployQueue,
// };

// pub async fn github_webhook(
//     body: web::Bytes,
//     installation_ids: InstallationStore,
//     redeploy_queue: web::Data<ReDeployQueue>,
// ) -> HttpResponse {
//     println!("Github webhook received");
//     println!(
//         "Github webhook received: {}",
//         String::from_utf8_lossy(&body)
//     );

//     let event = serde_json::from_slice::<EventType>(&body).unwrap();

//     match event {
//         EventType::Install(installation_event) => {
//             if installation_event.action == "created" {
//                 let url = format!(
//                     "https://github.com/{}",
//                     installation_event.repositories[0].full_name
//                 );

//                 println!("the key url is: {}", url);

//                 installation_ids
//                     .lock()
//                     .await
//                     .insert(url.clone(), installation_event);

//                 println!("Installation event stored");

//                 println!(
//                     "installation_ids from fn: {:?}",
//                     installation_ids.lock().await
//                 );
//             }
//         }
//         EventType::Push(redeploy_event) => {
//             redeploy_queue.publish(redeploy_event).await.unwrap();

//             println!("Installation event stored");

//             println!(
//                 "installation_ids from fn: {:?}",
//                 installation_ids.lock().await
//             );
//         }
//     }

//     HttpResponse::Ok().finish()
// }
