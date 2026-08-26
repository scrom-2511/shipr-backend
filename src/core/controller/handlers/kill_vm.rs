use actix_web::{HttpResponse, web};

use crate::{
    app_errors::AppError,
    core::app_types::{KillVmReq, LogsStore},
    core::controller::vm::{id_allocator::IdAllocator, vm_pool::VmPool},
    core::infra::kill_vm::kill_vm,
};

pub async fn kill_vm_handler(
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    let kill_vm_req = serde_json::from_slice::<KillVmReq>(&body).unwrap();

    println!("Kill VM request: {:?}", kill_vm_req);

    kill_vm(&kill_vm_req.project_id, &kill_vm_req.job_type, &vm_pool).await?;

    logs_store.lock().await.remove(&kill_vm_req.project_id);

    Ok(HttpResponse::Ok().finish())
}
