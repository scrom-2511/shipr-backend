use std::time::Duration;
use actix_web::web;
use redis::AsyncCommands;

use crate::{
    app::db::DbPool,
    core::{
        app_types::JobType,
        controller::{
            queue::idle_kill_queue::IdleKillQueue,
            storage::redis::Redis,
            vm::{id_allocator::IdAllocator, vm_pool::VmPool},
        },
        infra::kill_vm::kill_vm,
    },
};

const IDLE_TIMEOUT_SECS: i64 = 3600; // 1 hour idle timeout

pub async fn listen_idle_kill(
    idle_kill_queue: web::Data<IdleKillQueue>,
    redis: Redis,
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    pool: web::Data<DbPool>,
) {
    println!("Started listen_idle_kill listener worker");

    loop {
        let idle_req = match idle_kill_queue.pop_from_queue().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Idle kill queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let project_id = idle_req.project_id;
        let numeric_id = idle_req.numeric_id;

        loop {
            let mut redis_conn = redis.get_conn();
            let last_req_key = format!("project:last_request_time:{}", project_id);
            let start_time_key = format!("project:vm_start_time:{}", project_id);

            let now_ts = chrono::Utc::now().timestamp();
            let last_req_time: Option<i64> = redis_conn.get(&last_req_key).await.ok().flatten();

            let last_ts = last_req_time.unwrap_or(now_ts);
            let elapsed_idle = now_ts - last_ts;

            if elapsed_idle < IDLE_TIMEOUT_SECS {
                // Requests came in, so delay execution by the remaining time!
                let sleep_secs = (IDLE_TIMEOUT_SECS - elapsed_idle).max(1) as u64;
                println!(
                    "Project {} has recent activity (idle for {}s). Delaying kill job by {}s...",
                    project_id, elapsed_idle, sleep_secs
                );
                tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
            } else {
                // No request came for at least 1 hour! Time to kill VM & update active time in DB.
                println!(
                    "No requests received for project {} for 1 hour. Killing microVM and updating active time in DB.",
                    project_id
                );

                let start_time: Option<i64> = redis_conn.get(&start_time_key).await.ok().flatten();
                let active_seconds = if let Some(boot_ts) = start_time {
                    (now_ts - boot_ts).max(IDLE_TIMEOUT_SECS)
                } else {
                    IDLE_TIMEOUT_SECS
                };

                // 1. Update DB active time & status to 'stopped'
                let _ = sqlx::query(
                    r#"
                    UPDATE projects
                    SET active_seconds = COALESCE(active_seconds, 0) + $1,
                        status = 'stopped',
                        updated_at = NOW()
                    WHERE id = $2
                    "#,
                )
                .bind(active_seconds)
                .bind(numeric_id)
                .execute(pool.get_ref())
                .await;

                // 2. Kill the Firecracker microVM
                if let Err(e) = kill_vm(&project_id, &JobType::Run, &vm_pool, &id_allocator).await {
                    eprintln!("Failed to kill VM for project {}: {:?}", project_id, e);
                }

                // 3. Clean up Redis keys
                let _: () = redis_conn.del(&last_req_key).await.unwrap_or(());
                let _: () = redis_conn.del(&start_time_key).await.unwrap_or(());

                println!(
                    "Successfully killed idle VM for project {} and added {}s to active_seconds in DB.",
                    project_id, active_seconds
                );
                break;
            }
        }
    }
}
