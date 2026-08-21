use core::fmt;
use std::collections::HashMap;

use actix_web::web;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast::Sender};
pub type LogsStore = web::Data<Mutex<HashMap<String, Sender<String>>>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeployReq {
    pub project_id: String,
    pub envs: Vec<EnvVar>,
    pub branch: String,
    pub root_dir: String,
    pub full_name: String,
    pub installation_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShiprJson {
    pub install_commands: Option<Vec<String>>,
    pub build_commands: Option<Vec<String>>,
    pub run_commands: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeployDetails {
    pub root_dir: String,
    pub branch: Option<String>,
    pub full_name: String,
    pub project_id: String,
    pub presigned_upload_url: String,
    pub installation_access_token: String,
    pub envs: Option<Vec<EnvVar>>,
}

#[derive(Serialize, Deserialize)]
pub struct RedeployDetails {
    pub project_id: String,
    pub presigned_upload_url: String,
    pub presigned_download_url: String,
    pub access_token: String,
    pub commit_hash: String,
    pub branch: Option<String>,
    pub envs: Option<Vec<EnvVar>>,
}

#[derive(Serialize, Deserialize)]
pub struct RunDetails {
    pub presigned_download_url: String,
    pub project_id: String,
    pub envs: Option<Vec<EnvVar>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JobType {
    Deploy,
    Run,
    Redeploy,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, sqlx::Type, Copy)]
#[sqlx(type_name = "project_type", rename_all = "lowercase")]
pub enum ProjectType {
    Html,
    Rust,
    React,
    Node,
    Unknown,
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProjectType::Html => "html",
            ProjectType::Rust => "rust",
            ProjectType::React => "react",
            ProjectType::Node => "node",
            ProjectType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KillVmReq {
    pub project_id: String,
    pub job_type: JobType,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct JobCompletedReq {
    pub project_id: String,
    pub job_type: JobType,
    pub commit_hash: Option<String>,
    pub project_type: ProjectType,
    pub branch: Option<String>,
}
