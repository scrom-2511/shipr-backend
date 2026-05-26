use reqwest::Client;
use serde_json::json;

use crate::{app_errors::AppError, core::app_types::{JobType, ProjectType}};

pub struct Host {
    client: Client,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn kill_vm(&self, project_id: String, job_type: JobType) -> Result<(), AppError> {
        self.client
            .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/kill-vm")
            .json(&json!({
                "project_id": project_id,
                "job_type": job_type,
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn send_logs(&self, project_id: &str, log: &str) -> Result<(), AppError> {
        self.client
            .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/send-logs")
            .json(&json!({
                "project_id": project_id,
                "log": log,
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn job_completed(
        &self,
        project_id: String,
        job_type: JobType,
        commit_hash: Option<String>,
        project_type: ProjectType,
    ) -> Result<(), AppError> {
        self.client
            .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/job-completed")
            .json(&json!({
                "project_id": project_id,
                "job_type": job_type,
                "commit_hash": commit_hash,
                "project_type": project_type,
            }))
            .send()
            .await?;
        Ok(())
    }
}
