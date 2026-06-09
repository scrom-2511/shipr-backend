
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use aws_sdk_s3::presigning::PresigningConfigError;
use sqlx::error::Error as SqlxError;

use crate::app::controllers::ApiResponse;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("GitHub OAuth error: {0}")]
    GithubOAuthError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Presigning config error: {0}")]
    PresigningConfigError(#[from] PresigningConfigError),

    #[error("S3 SDK error: {0}")]
    Sdk(#[from] aws_sdk_s3::Error),

    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("SQLx error: {0}")]
    SqlxError(#[from] SqlxError),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Json Web Token error: {0}")]
    JsonWebToken(#[from] jsonwebtoken::errors::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Tokio mpsc error: {0}")]
    TokioMpscError(#[from] tokio::sync::broadcast::error::SendError<std::string::String>),

    #[error("hickory_proto::serialize::binary::DecodeError: {0}")]
    HickoryProtoDecodeError(#[from] hickory_proto::serialize::binary::DecodeError),

    #[error("hickory_proto::error::ProtoError: {0}")]
    HickoryProtoError(#[from] hickory_proto::ProtoError),

    #[error("Lapin error: {0}")]
    LapinError(String),

    #[error("Channel creation error: {0}")]
    ChannelError(String),

    #[error("Queue declaration error: {0}")]
    QueueError(String),

    #[error("Invalid git url")]
    InvalidGitUrl,

    #[error("Gitclone failed: {0}")]
    GitCloneFailed(String),

    #[error("Error creating dir: {0}")]
    DirCreationFailed(String),

    #[error("Running command failed: {0}")]
    CmdFailed(String),

    #[error("Failed getting the current working directory: {0}")]
    CurrentWorkingDirUnavailable(String),

    #[error("Failed to start firecracker: {0}")]
    StartingFirecrackerFailed(String),

    #[error("Reload the page")]
    ReloadPage,

    #[error("Failed to get connection from pool: {0}")]
    FailedToGetConnectionFromPool(String),

    #[error("VM not ready")]
    VmNotReady,

    #[error("Unknown project type")]
    UnknownProjectType,

    #[error("ID allocation failed: {0}")]
    IdAllocationFailed(String),

    #[error("Failed to get id from pool: {0}")]
    FailedToGetIdFromPool(String),

    #[error("HTTP client build failed: {0}")]
    HttpClientBuildFailed(String),

    #[error("Invalid project id: {0}")]
    InvalidProjectId(String),

    #[error("VM provisioning failed: {0}")]
    VmProvisioningFailed(String),

    #[error("Request forwarding failed: {0}")]
    RequestForwardingFailed(String),

    #[error("Response read failed: {0}")]
    ResponseReadFailed(String),

    #[error("Method conversion failed: {0}")]
    MethodConversionFailed(String),

    #[error("No available VM")]
    NoAvailableVm,

    #[error("Starting server failed: {0}")]
    StartingServerFailed(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Password too short")]
    PasswordTooShort,

    #[error("Password hashing failed")]
    PasswordHashFailed,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Internal server error")]
    InternalServerError,

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InvalidGitUrl => StatusCode::BAD_REQUEST,
            AppError::UnknownProjectType => StatusCode::BAD_REQUEST,
            AppError::InvalidEmail => StatusCode::BAD_REQUEST,
            AppError::PasswordTooShort => StatusCode::BAD_REQUEST,
            AppError::UserAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::UserNotFound => StatusCode::NOT_FOUND,
            AppError::GithubOAuthError(_) => StatusCode::BAD_REQUEST,

            AppError::IdAllocationFailed(_)
            | AppError::FailedToGetIdFromPool(_)
            | AppError::StartingFirecrackerFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,

            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let response = ApiResponse::<()> {
            success: false,
            message: self.to_string(),
            data: None,
        };
        HttpResponse::build(self.status_code()).json(response)
    }
}
