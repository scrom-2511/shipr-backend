use std::time::Duration;

use actix_web::web;
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    app::{
        controllers::billing::onDemand::auto_top_up::auto_top_up,
        db::DbPool,
        models::{projects, users},
        state::AppState,
    },
    app_errors::AppError,
    core::{
        controller::{storage::redis::Redis, vm::vm_pool::VmPool},
        infra::kill_vm::kill_vm,
    },
};

async fn kill_idle_vms(
    redis: &Redis,
    vm_pool: &VmPool,
    pool: &DbPool,
    state: web::Data<AppState>,
) -> Result<(), AppError> {
    let mut redis_conn = redis.get_conn();
    let now = chrono::Utc::now().timestamp();
    let cutoff = now - 120;

    let idle_vms: Vec<String> = redis_conn
        .zrangebyscore("project:last_request_time", "-inf", cutoff)
        .await?;

    for project_id in idle_vms {
        let start_time = redis_conn
            .get::<String, i64>(format!("project:vm_start_time:{}", project_id.clone()))
            .await
            .unwrap();

        let total_active_time = now - start_time;

        println!(
            "Total active time for project: {} at {}",
            project_id, total_active_time
        );

        let mut user_id = -1;

        if let Ok(Some(project)) = projects::Entity::find()
            .filter(projects::Column::ProjectId.eq(project_id.clone()))
            .one(pool)
            .await
        {
            user_id = project.user_id;
            let new_active_seconds = project.active_seconds + total_active_time;
            let mut active_project: projects::ActiveModel = project.into();
            active_project.active_seconds = Set(new_active_seconds);
            active_project.status = Set(crate::app::models::ProjectStatus::Stopped);
            active_project.updated_at = Set(chrono::Utc::now().naive_utc());
            let _ = active_project.update(pool).await;
        }

        let mut credits_balance_to_subtract = (total_active_time as f64 * 0.00000556) as i64;
        if credits_balance_to_subtract < 1 {
            credits_balance_to_subtract = 2;
        }

        println!(
            "Credits balance to subtract: {}",
            credits_balance_to_subtract
        );

        let mut user_credit_balance = -1;
        let mut auto_topup_enabled = false;

        if let Ok(Some(user)) = users::Entity::find_by_id(user_id).one(pool).await {
            let mut active_user: users::ActiveModel = user.clone().into();

            user_credit_balance = user.credit_balance;
            auto_topup_enabled = user.auto_topup_enabled;

            active_user.credit_balance = Set(user.credit_balance - credits_balance_to_subtract);

            let _ = active_user.update(pool).await;
        }

        println!(
            "Removed {} credits from user: {}",
            credits_balance_to_subtract, user_id
        );

        let _: () = redis_conn
            .del(format!("project:vm_start_time:{}", project_id))
            .await
            .unwrap();

        println!("Killed vm for project: {}", project_id);

        kill_vm(&project_id, &crate::core::app_types::JobType::Run, vm_pool).await?;

        if user_credit_balance < 1000 && auto_topup_enabled {
            auto_top_up(state.clone(), pool, user_id).await?;
        }
    }

    Ok(())
}

pub async fn run_kill_idle_vms(
    redis: &Redis,
    vm_pool: &VmPool,
    pool: &DbPool,
    state: web::Data<AppState>,
) {
    loop {
        kill_idle_vms(redis, vm_pool, pool, state.clone())
            .await
            .unwrap();
        // tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
