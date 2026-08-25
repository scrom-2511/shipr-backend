use actix_web::web;
use lapin::options::BasicAckOptions;
use redis::AsyncCommands;
use std::time::Duration;

use crate::app::models::{projects, users};
use crate::{
    app::db::DbPool,
    core::{
        app_types::JobType,
        controller::{
            queue::idle_kill_queue::{IdleKillQueue, IdleKillReq},
            storage::redis::Redis,
            vm::{id_allocator::IdAllocator, vm_pool::VmPool},
        },
        infra::kill_vm::kill_vm,
    },
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

const IDLE_TIMEOUT_SECS: i64 = 120; // 2 minutes idle timeout for testing

pub async fn listen_idle_kill(
    idle_kill_queue: web::Data<IdleKillQueue>,
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    pool: web::Data<DbPool>,
    redis: web::Data<Redis>,
) {
    println!("Started listen_idle_kill listener worker");

    loop {
        let delivery = match idle_kill_queue.pop_from_queue().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Idle kill queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let idle_req = match serde_json::from_slice::<IdleKillReq>(&delivery.data) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to deserialize message: {:?}", e);

                delivery.ack(BasicAckOptions::default()).await.ok();

                continue;
            }
        };

        let mut redis_conn = redis.get_conn();
        let last_activity = match redis_conn
            .get::<String, i64>(format!("project:last_request_time:{}", idle_req.project_id))
            .await
        {
            Ok(timestamp) => timestamp,

            Err(e) => {
                eprintln!(
                    "Failed to get last activity for {}: {:?}",
                    idle_req.project_id, e
                );

                continue;
            }
        };

        let now = chrono::Utc::now().timestamp();

        if now - last_activity + 20 >= IDLE_TIMEOUT_SECS {
            kill_vm(&idle_req.project_id, &JobType::Run, &vm_pool, &id_allocator)
                .await
                .ok()
                .unwrap();

            println!("Killed vm for project: {} at {}", idle_req.project_id, now);

            let project_id_int = idle_req.project_id_int;

            let start_time = redis
                .get_conn()
                .get::<String, i64>(format!("project:vm_start_time:{}", idle_req.project_id))
                .await
                .unwrap();

            let total_active_time = now - start_time;

            println!(
                "Total active time for project: {} at {}",
                idle_req.project_id, total_active_time
            );

            let mut user_id = -1;

            if let Ok(Some(project)) = projects::Entity::find_by_id(project_id_int)
                .one(pool.get_ref())
                .await
            {
                user_id = project.user_id;
                let new_active_seconds = project.active_seconds + total_active_time;
                let mut active_project: projects::ActiveModel = project.into();
                active_project.active_seconds = Set(new_active_seconds);
                active_project.status = Set(crate::app::models::ProjectStatus::Stopped);
                active_project.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
                let _ = active_project.update(pool.get_ref()).await;
            }

            let credits_balance_to_subtract = (total_active_time as f64 * 0.00000556) as i64;

            if let Ok(Some(user)) = users::Entity::find_by_id(user_id).one(pool.get_ref()).await {
                let mut active_user: users::ActiveModel = user.clone().into();

                active_user.credit_balance = Set(user.credit_balance - credits_balance_to_subtract);

                let _ = active_user.update(pool.get_ref()).await;
            }

            let _: () = redis_conn
                .del(format!("project:vm_start_time:{}", idle_req.project_id))
                .await
                .unwrap();

            let _: () = redis_conn
                .del(format!("project:last_request_time:{}", idle_req.project_id))
                .await
                .unwrap();
        } else {
            println!(
                "Project {} is still active, pushing back to queue at {}",
                idle_req.project_id, now
            );
            println!(
                "Project {} is still active, pushing back to queue at {}",
                idle_req.project_id, now
            );
            println!(
                "Project {} is still active, pushing back to queue at {}",
                idle_req.project_id, now
            );

            let _: () = idle_kill_queue
                .add_to_queue(&IdleKillReq {
                    project_id: idle_req.project_id.clone(),
                    project_id_int: idle_req.project_id_int,
                })
                .await
                .unwrap();
        }

        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
            eprintln!("Failed to ACK message: {:?}", e);
        }
    }
}
