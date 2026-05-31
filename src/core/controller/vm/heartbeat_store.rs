use std::time::Duration;

use redis::AsyncCommands;

use crate::app_errors::AppError;
use crate::core::controller::storage::redis::Redis;

#[derive(Clone)]
pub struct HeartbeatStore {
    redis: Redis,
}

impl HeartbeatStore {
    pub fn new(redis: Redis) -> Self {
        Self { redis }
    }

    fn heartbeat_key(&self, project_id: &str) -> String {
        format!("heartbeat:{}", project_id)
    }

    pub async fn heartbeat(&self, project_id: &str, ttl: Duration) -> Result<(), AppError> {
        let key = self.heartbeat_key(project_id);

        let mut conn = self.redis.get_conn();

        let _: () = conn.set_ex(key, "alive", ttl.as_secs()).await?;

        Ok(())
    }

    pub async fn is_dead(&self, project_id: &str) -> Result<bool, AppError> {
        let key = self.heartbeat_key(project_id);

        let mut conn = self.redis.get_conn();

        let exists: bool = conn.exists(key).await?;

        Ok(!exists)
    }

    pub async fn remove(&self, project_id: &str) -> Result<(), AppError> {
        let key = self.heartbeat_key(project_id);

        let mut conn = self.redis.get_conn();

        let _: () = conn.del(key).await?;

        Ok(())
    }
}
