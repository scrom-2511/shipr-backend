use crate::app::db::DbPool;
use crate::app::models::{project_traffic, projects, users};
use crate::app::state::AppState;
use crate::app_errors::AppError;
use crate::core::app_types::JobType;
use crate::core::controller::dispatcher::job_dispatcher::JobDispatcher;
use crate::core::controller::queue::idle_kill_queue::{IdleKillQueue, IdleKillReq};
use crate::core::controller::storage::redis::Redis;
use crate::core::controller::vm::heartbeat_store::HeartbeatStore;
use crate::core::controller::vm::vm_pool::VmPool;
use actix_web::http::Uri;
use actix_web::{App, HttpRequest, HttpResponse, web};
use redis::AsyncCommands;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client, Method};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;
use url::Url;

#[derive(Clone)]
pub struct VmRequestProxy {
    vm_pool: VmPool,
    client: Client,
    job_dispatcher: JobDispatcher,
    heartbeat_store: HeartbeatStore,
    pool: DbPool,
    redis: Redis,
    idle_kill_queue: IdleKillQueue,
    state: web::Data<AppState>,
}

impl VmRequestProxy {
    pub fn new(
        vm_pool: VmPool,
        job_dispatcher: JobDispatcher,
        heartbeat_store: HeartbeatStore,
        pool: DbPool,
        redis: Redis,
        idle_kill_queue: IdleKillQueue,
        state: web::Data<AppState>,
    ) -> Result<Self, AppError> {
        let client = Client::new();

        Ok(Self {
            vm_pool,
            client,
            job_dispatcher,
            heartbeat_store,
            pool,
            redis,
            idle_kill_queue,
            state,
        })
    }

    async fn extract_project_and_path(
        &self,
        req: &HttpRequest,
    ) -> Result<(String, i32, Uri), AppError> {
        let host = req.connection_info().host().to_owned();

        let uri = req.uri().to_owned();

        let name = host.split(".").next().unwrap().to_owned();

        println!("name is: {}", name);

        let mut conn = self.redis.get_conn();

        let cache_key = format!("project_name_to_id_v2:{}", name);

        let cached: Option<String> = conn.get(&cache_key).await?;

        if let Some(val) = cached
            && let Some((project_id_str, project_id_int_str)) = val.split_once(':')
            && let Ok(project_id_int) = project_id_int_str.parse::<i32>()
        {
            println!(
                "Found project ids in redis: {} (slug), {} (id) for name: {}",
                project_id_str, project_id_int, name
            );
            return Ok((project_id_str.to_string(), project_id_int, uri));
        }

        let project = crate::app::models::projects::Entity::find()
            .filter(crate::app::models::projects::Column::ProjectId.eq(&name))
            .one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::Database(format!("Project with name {} not found", name)))?;

        let project_id_str = project.project_id;
        let project_id_int = project.id;

        let cache_val = format!("{}:{}", project_id_str, project_id_int);
        let _: () = conn.set_ex(&cache_key, &cache_val, 3600).await?;

        println!(
            "Found project ids in db: {} (slug), {} (id) for name: {} and cached it",
            project_id_str, project_id_int, name
        );

        Ok((project_id_str, project_id_int, uri))
    }

    fn build_target_url(&self, base_id: u8, uri: Uri) -> Url {
        let path = uri.path().trim_start_matches('/');
        let vm_ip = format!("172.16.0.{}", base_id + 2);
        let target_url_str = format!("http://{}:3000/{}", vm_ip, path);

        Url::from_str(&target_url_str).unwrap()
    }

    async fn forward_request(
        &self,
        req: &HttpRequest,
        body: web::Bytes,
        target_url: Url,
    ) -> HttpResponse {
        let method = req
            .method()
            .as_str()
            .parse::<Method>()
            .unwrap_or(Method::GET);

        let mut forward_req = self.client.request(method, target_url);

        for (name, value) in req.headers().iter() {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_str().as_bytes())
                && let Ok(header_value) = HeaderValue::from_bytes(value.as_bytes())
            {
                forward_req = forward_req.header(header_name, header_value);
            }
        }

        let resp = forward_req.body(body).send().await;

        match resp {
            Ok(upstream) => {
                let status = actix_web::http::StatusCode::from_u16(upstream.status().as_u16())
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

                let mut response = HttpResponse::build(status);

                for (name, value) in upstream.headers().iter() {
                    if let Ok(value_str) = value.to_str() {
                        response.insert_header((name.as_str(), value_str));
                    }
                }

                match upstream.bytes().await {
                    Ok(bytes) => response.body(bytes),
                    Err(_) => {
                        HttpResponse::InternalServerError().body("Failed to read upstream body")
                    }
                }
            }
            Err(_) => HttpResponse::BadGateway().body("Upstream request failed"),
        }
    }

    pub async fn proxy_request(
        &mut self,
        req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse, AppError> {
        let (project_id, project_id_int, target_path) = self.extract_project_and_path(&req).await?;

        if let Err(e) = self.idle_vm_manager(&project_id).await {
            println!(
                "Warning: Failed to update idle vm manager for project {}: {}",
                project_id_int, e
            );
        }

        if let Err(e) = self.track_traffic(&project_id).await {
            println!(
                "Warning: Failed to track traffic for project {}: {}",
                project_id_int, e
            );
        }

        self.job_dispatcher.dispatch_run_job(&project_id).await?;

        println!("Job dispatched");

        let (vm, _) = self
            .vm_pool
            .get_or_create_vm(&project_id, &JobType::Run)
            .await
            .map_err(|_| AppError::ReloadPage)?;

        let base_id = vm.get_base_id();

        self.wait_for_port(base_id, 3000, Duration::from_secs(30))
            .await?;

        let target_url = self.build_target_url(base_id, target_path);

        let resp = self.forward_request(&req, body, target_url).await;

        Ok(resp)
    }

    async fn track_traffic(&self, project_id: &str) -> Result<(), AppError> {
        let project = project_traffic::Entity::find()
            .filter(project_traffic::Column::ProjectId.eq(project_id))
            .one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match project {
            Some(project) => {
                let mut active: project_traffic::ActiveModel = project.clone().into();

                active.request_count = Set(project.request_count + 1);

                active
                    .update(&self.pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }

            None => {
                return Err(AppError::Database(format!(
                    "Project {} not found",
                    project_id
                )));
            }
        }

        Ok(())
    }

    async fn idle_vm_manager(&self, project_id: &str) -> Result<(), AppError> {
        let start_time_key = format!("project:vm_start_time:{}", project_id);

        let last_activity_key = format!("project:last_request_time");

        let now = chrono::Utc::now().timestamp();

        let mut redis_conn = self.redis.get_conn();

        let existing_start_time: Option<i64> = redis_conn.get(&start_time_key).await?;

        if existing_start_time.is_none() {
            let _: () = redis_conn.set(&start_time_key, now).await?;
        }

        let _: () = redis_conn.zadd(&last_activity_key, project_id, now).await?;

        Ok(())
    }

    async fn wait_for_port(
        &self,
        base_id: u8,
        port: u16,
        max_wait: Duration,
    ) -> Result<(), AppError> {
        let vm_ip = format!("172.16.0.{}", base_id + 2);
        let addr = format!("{}:{}", vm_ip, port);

        let deadline = tokio::time::Instant::now() + max_wait;

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(AppError::StartingFirecrackerFailed(format!(
                    "Timed out waiting for port {}",
                    addr
                )));
            }
            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    println!("Port ready: {}", addr);
                    return Ok(());
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    }
}
