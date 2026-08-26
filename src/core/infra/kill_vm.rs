use crate::app_errors::AppError;
use crate::core::app_types::JobType;
use crate::core::controller::vm::firecracker::Firecracker;
use crate::core::controller::vm::vm_pool::VmPool;

pub async fn kill_vm(
    project_id: &str,
    job_type: &JobType,
    vm_pool: &VmPool,
) -> Result<(), AppError> {
    let vm_id = match vm_pool.get_from_pool(project_id, job_type).await? {
        Some(id) => id,
        None => {
            println!(
                "No active VM found for project: {} and job_type: {:?}",
                project_id, job_type
            );
            return Ok(());
        }
    };

    println!("Killing VM: {}", vm_id);

    let new_vm = Firecracker::new_from_vm_id(vm_id);

    new_vm.destroy_vm().await?;

    println!("Killed VM: {}", vm_id);

    vm_pool
        .remove_from_pool(project_id, job_type, vm_id)
        .await?;

    println!("Removed VM: {}", vm_id);

    Ok(())
}
