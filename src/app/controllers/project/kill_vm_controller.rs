use crate::app_errors::AppError;
use crate::core::app_types::KillVmReq;
use crate::core::controller::vm::vm_pool::VmPool;
use crate::core::infra::kill_vm::kill_vm;
use actix_web::{HttpResponse, web};
use serde_json::json;

pub async fn kill_vm_controller(
    body: web::Json<KillVmReq>,
    vm_pool: web::Data<VmPool>,
) -> Result<HttpResponse, AppError> {
    println!("kill_vm_controller called");
    kill_vm(&body.project_id, &body.job_type, &vm_pool).await?;
    Ok(HttpResponse::Ok().json(json!({"message": "VM killed"})))
}
