use actix_web::{HttpResponse, web};
use serde::Deserialize;

use crate::{
    app::{controllers::ApiResponse, db::DbPool},
    app_errors::AppError,
};

#[derive(Debug, Deserialize)]
pub struct EditProjectBody {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub root_dir: String,
    pub dist_dir: String,
    pub install_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
}

pub async fn edit_project_details_controller(
    pool: web::Data<DbPool>,
    body: web::Json<EditProjectBody>,
) -> Result<HttpResponse, AppError> {
    let query = "UPDATE projects SET url = $2, project_id = $1, branch = $3, root_dir = $4, dist_dir = $5, install_cmds = $6, build_cmds = $7, run_cmds = $8 WHERE id = $9";

    sqlx::query(query)
        .bind(&body.name)
        .bind(&body.url)
        .bind(&body.branch)
        .bind(&body.root_dir)
        .bind(&body.dist_dir)
        .bind(&body.install_cmds)
        .bind(&body.build_cmds)
        .bind(&body.run_cmds)
        .bind(body.id)
        .execute(pool.as_ref())
        .await
        .map_err(|e| {
            println!("DB ERROR: {:?}", e);
            AppError::Database(e.to_string())
        })?;

    Ok(HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: "Successfully updated the project details".to_string(),
        data: None,
    }))
}
