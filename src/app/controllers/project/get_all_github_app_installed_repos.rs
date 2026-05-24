use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Serialize;

use crate::{
    app::{
        controllers::ApiResponse, db::DbPool, middlewares::AuthMiddleware,
        models::GithubAppInstallation,
    },
    app_errors::AppError,
    shared::github_app::GithubApp,
};

#[derive(Serialize)]
struct GithubAppInstalledRepo {
    id: i32,
    name: String,
    full_name: String,
    installation_id: i32,
}

#[derive(Serialize)]
struct GithubAppInstalledReposResponse {
    repos: Vec<GithubAppInstalledRepo>,
}

pub async fn get_all_github_app_installed_repos(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let user_id = req.extensions().get::<AuthMiddleware>().unwrap().user_id;

    let query = r#"SELECT installation_ids, id, user_id FROM github_repos WHERE user_id = $1"#;

    let github_app_installations: Vec<GithubAppInstallation> = sqlx::query_as(query)
        .bind(user_id)
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e| {
            println!("DB ERROR: {:?}", e);
            AppError::InternalServerError
        })?;

    println!("github_app_installations: {:?}", github_app_installations);

    let github = GithubApp::new();
    let mut all_repos = Vec::new();

    for installation in github_app_installations {
        for id in installation.installation_ids {
            let repos = github.get_user_installed_repos(id as u32).await?;
            let repos = repos
                .iter()
                .map(|repo| GithubAppInstalledRepo {
                    id: repo.id,
                    name: repo.name.clone(),
                    full_name: repo.full_name.clone(),
                    installation_id: id,
                })
                .collect::<Vec<GithubAppInstalledRepo>>();
            all_repos.extend(repos);
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "All GitHub app installed repos fetched successfully".to_string(),
        data: Some(GithubAppInstalledReposResponse { repos: all_repos }),
    }))
}
