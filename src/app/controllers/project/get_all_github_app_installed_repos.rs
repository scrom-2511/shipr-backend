use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

use crate::{
    app::{
        controllers::ApiResponse, db::DbPool, middlewares::AuthMiddleware, models::github_repos,
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

    let github_app_installations = github_repos::Entity::find()
        .filter(github_repos::Column::UserId.eq(user_id))
        .all(pool.as_ref())
        .await
        .map_err(|e| {
            println!("DB ERROR: {:?}", e);
            AppError::InternalServerError
        })?;

    println!("github_app_installations: {:?}", github_app_installations);

    let github = GithubApp::new();
    let mut all_repos = Vec::new();

    for installation in github_app_installations {
        if let Some(installation_ids) = installation.installation_ids {
            for id in installation_ids {
                let installation_access_token =
                    github.get_installation_access_token(id as u64).await?;

                let repos = github
                    .get_user_installed_repos(&installation_access_token)
                    .await?;

                for repo in repos {
                    all_repos.push(GithubAppInstalledRepo {
                        id: repo.id,
                        name: repo.name.clone(),
                        full_name: repo.full_name.clone(),
                        installation_id: id,
                    });
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "All GitHub app installed repos fetched successfully".to_string(),
        data: Some(GithubAppInstalledReposResponse { repos: all_repos }),
    }))
}
