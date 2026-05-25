use actix_web::{HttpRequest, HttpResponse, web};

use crate::{
    app::{
        controllers::ApiResponse,
        db::DbPool,
        webhooks::{
            github_installation::{self, GithubAppWebhookPayload, github_webhook_installation},
            github_push::{GithubPushEvent, github_webhook_push},
        },
    },
    app_errors::AppError,
    core::controller::queue::redeploy_queue::ReDeployQueue,
};

pub async fn github_event(
    body: web::Bytes,
    pool: web::Data<DbPool>,
    req: HttpRequest,
    redeploy_queue: web::Data<ReDeployQueue>,
) -> Result<HttpResponse, AppError> {
    println!("body: {:?}", body);

    let event_header = req
        .headers()
        .get("X-GitHub-Event")
        .unwrap()
        .to_str()
        .unwrap();

    match event_header {
        "installation" => {
            let body = serde_json::from_slice::<GithubAppWebhookPayload>(&body).unwrap();
            Ok(github_webhook_installation(web::Json(body), pool).await?)
        }

        "push" => {
            let body = serde_json::from_slice::<GithubPushEvent>(&body).unwrap();
            Ok(github_webhook_push(web::Json(body), pool, redeploy_queue).await?)
        }
        _ => {
            return Ok(HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "Installation not created".to_string(),
                data: None,
            }));
        }
    }
}
