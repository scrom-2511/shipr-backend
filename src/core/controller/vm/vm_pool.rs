use redis::AsyncCommands;

use crate::{
    app_errors::AppError,
    core::{
        app_types::JobType,
        controller::{
            storage::redis::Redis,
            vm::{firecracker::Firecracker, id_allocator::IdAllocator},
        },
    },
};

#[derive(Clone)]
pub struct VmPool {
    redis: Redis,
    id_allocator: IdAllocator,
}

const IDEAL_VMS_QUEUE: &str = "ideal_vms:queue";
const IDEAL_VMS_SEEN: &str = "ideal_vms:seen";

impl VmPool {
    pub fn new(redis: Redis, id_allocator: IdAllocator) -> Self {
        Self {
            redis,
            id_allocator,
        }
    }

    pub async fn add_to_pool(
        &self,
        project_id: &str,
        job_type: &JobType,
        vm_id: u8,
    ) -> Result<(), AppError> {
        let project_id = &format!("{}_{}", project_id, job_type);

        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.set(project_id, vm_id).await?;

        Ok(())
    }

    pub async fn get_from_pool(
        &self,
        project_id: &str,
        job_type: &JobType,
    ) -> Result<Option<u8>, AppError> {
        let project_id = &format!("{}_{}", project_id, job_type);

        let mut conn = self.redis.get_conn().await?;

        let vm_id = conn.get(project_id).await?;

        Ok(vm_id)
    }

    pub async fn remove_from_pool(
        &self,
        project_id: &str,
        job_type: &JobType,
    ) -> Result<(), AppError> {
        println!("Removing from pool: {}", project_id);
        let project_id = &format!("{}_{}", project_id, job_type);

        let mut conn = self.redis.get_conn().await?;

        let _: () = conn.del(project_id).await?;

        Ok(())
    }

    pub async fn add_to_ideal_vms(&self, vm_id: u8) -> Result<(), AppError> {
        let mut conn = self.redis.get_conn().await?;

        let added: bool = conn.sadd(IDEAL_VMS_SEEN, vm_id).await?;

        if added {
            let _: () = conn.rpush(IDEAL_VMS_QUEUE, vm_id).await?;
        }

        Ok(())
    }

    pub async fn get_from_ideal_vms(&self) -> Result<Option<u8>, AppError> {
        let mut conn = self.redis.get_conn().await?;

        let vm_id: Option<u8> = conn.lpop(IDEAL_VMS_QUEUE, None).await?;

        if let Some(id) = vm_id {
            let _: () = conn.srem(IDEAL_VMS_SEEN, id).await?;
        }

        Ok(vm_id)
    }

    pub async fn get_or_create_vm(
        &self,
        project_id: &str,
        job_type: JobType,
    ) -> Result<(Firecracker, bool), AppError> {
        let something = self.get_from_pool(project_id, &job_type).await?;

        match something {
            Some(id) => {
                let vm = Firecracker::new_from_vm_id(id);

                println!("VM found in pool: {}", id);
                Ok((vm, false))
            }
            None => {
                let vm_id = self.get_from_ideal_vms().await?.ok_or_else(|| {
                    AppError::QueueError("No ideal VMs available in pool".to_string())
                })?;
                let new_vm = Firecracker::new_from_vm_id(vm_id);

                Ok((new_vm, true))
            }
        }
    }
}
