use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, prelude::Type};

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "project_status", rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Deploying,
    Error,
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub project_id: String,

    pub repo_id: i32,

    pub install_cmds: Option<Vec<String>>,
    pub run_cmds: Option<Vec<String>>,
    pub build_cmds: Option<Vec<String>>,

    pub dist_dir: String,
    pub home_dir: String,
    pub url: String,

    pub user_id: i32,

    pub commit_hash: String,

    pub last_deployment_time: chrono::DateTime<chrono::Utc>,
    pub status: ProjectStatus,

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, FromRow, Deserialize, Serialize)]
pub struct GithubAppInstallation {
    pub id: i32,
    pub user_id: Option<i32>,
    pub installation_ids: Vec<i32>,
}
