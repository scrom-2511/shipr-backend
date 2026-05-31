use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{Client, Method};
use serde::Serialize;

use crate::{
    app::webhooks::github_installation::{
        GithubInstallationRepositoriesResponse, GithubRepository,
    },
    app_errors::AppError,
};

#[derive(Serialize)]
struct Claims {
    iat: u64,
    exp: u64,
    iss: u64,
}

pub struct GithubApp {
    client: Client,
}

impl Default for GithubApp {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubApp {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn generate_jwt(&self) -> Result<String, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            iat: now - 60,
            exp: now + 600,
            iss: 3566236,
        };

        let key_str = fs::read_to_string("shipr-deployment.pem")?;
        let key = EncodingKey::from_rsa_pem(key_str.as_bytes())?;

        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

        Ok(token)
    }

    async fn using_app_jwt_req(
        &self,
        method: Method,
        url: &str,
    ) -> Result<reqwest::Response, AppError> {
        let jwt = self.generate_jwt()?;

        let res = self
            .client
            .request(method, url)
            .bearer_auth(jwt)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
    }

    async fn using_access_token_req(
        &self,
        method: Method,
        url: &str,
        installation_access_token: &str,
    ) -> Result<reqwest::Response, AppError> {
        let res = self
            .client
            .request(method, url)
            .bearer_auth(installation_access_token)
            .header("User-Agent", "shipr-deployment")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        Ok(res)
    }

    pub async fn get_installation_access_token(
        &self,
        installation_id: u64,
    ) -> Result<String, AppError> {
        let url = format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            installation_id
        );

        println!("URL: {}", url);

        let res = self.using_app_jwt_req(Method::POST, &url).await?;
        let json = res.json::<serde_json::Value>().await?;

        println!("JSON: {}", json);

        let token = json["token"].as_str().unwrap();

        Ok(token.to_string())
    }

    pub async fn get_user_installed_repos(
        &self,
        installation_access_token: &str,
    ) -> Result<Vec<GithubRepository>, AppError> {
        let url = "https://api.github.com/installation/repositories";

        let res = self
            .using_access_token_req(Method::GET, url, installation_access_token)
            .await?;

        let json = res.json::<GithubInstallationRepositoriesResponse>().await?;

        let repos = json.repositories;

        Ok(repos)
    }

    async fn get_default_branch(
        &self,
        owner: &str,
        repo: &str,
        installation_access_token: &str,
    ) -> Result<String, AppError> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

        let res = self
            .using_access_token_req(Method::GET, &url, installation_access_token)
            .await?;

        let json = res.json::<serde_json::Value>().await?;

        let branch = json["default_branch"].as_str().unwrap();

        Ok(branch.to_string())
    }

    pub async fn get_commit_sha(
        &self,
        branch: Option<String>,
        owner: &str,
        repo: &str,
        installation_access_token: &str,
    ) -> Result<(String, String), AppError> {
        let branch = if branch.is_none() {
            self.get_default_branch(owner, repo, installation_access_token)
                .await?
        } else {
            branch.unwrap().to_string()
        };

        let url = format!(
            "https://api.github.com/repos/{}/{}/commits/{}",
            owner, repo, branch
        );

        let res = self
            .using_access_token_req(Method::GET, &url, installation_access_token)
            .await?;
        let json = res.json::<serde_json::Value>().await?;

        let commit_hash = json["sha"].as_str().unwrap();

        Ok((commit_hash.to_string(), branch))
    }

    pub async fn get_tarball_url(
        &self,
        branch: Option<String>,
        owner: &str,
        repo: &str,
        installation_access_token: &str,
    ) -> Result<String, AppError> {
        let (commit_hash, _) = self
            .get_commit_sha(branch, owner, repo, installation_access_token)
            .await?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/tarball/{}",
            owner, repo, commit_hash
        );

        Ok(url)
    }

    pub async fn get_tarball_url_from_commit_hash(
        &self,
        commit_hash: &str,
        owner: &str,
        repo: &str,
    ) -> Result<String, AppError> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/tarball/{}",
            owner, repo, commit_hash
        );

        Ok(url)
    }
}
