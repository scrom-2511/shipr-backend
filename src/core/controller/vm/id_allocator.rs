use redis::AsyncCommands;

use crate::{app_errors::AppError, core::controller::storage::redis::Redis};

const MAX_IDS: u8 = 64;

#[derive(Clone)]
pub struct IdAllocator {
    redis: Redis,
}

const CURRENT_IDS: &str = "current_ids";

impl IdAllocator {
    pub fn new(redis: Redis) -> Self {
        Self { redis }
    }

    async fn remove_from_current_ids(&self, id: u8) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn();

        let _: () = conn.srem(CURRENT_IDS, id).await?;

        Ok(())
    }

    pub async fn allocate_id(&self) -> Result<u8, AppError> {
        let mut conn = self.redis.get_conn();

        for id in 1..MAX_IDS {
            let inserted: u8 = conn.sadd(CURRENT_IDS, id).await?;

            if inserted == 1 {
                return Ok(id);
            }
        }

        Err(AppError::StartingFirecrackerFailed(
            "No available subnet IDs".to_string(),
        ))
    }

    pub async fn release_id(&self, id: u8) -> Result<(), AppError> {
        println!("Releasing ID: {}", id);
        self.remove_from_current_ids(id).await?;

        Ok(())
    }
}
