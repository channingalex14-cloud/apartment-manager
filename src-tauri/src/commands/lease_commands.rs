//! 合同命令

use crate::db::{get_app_context, queries};
use crate::errors::app_error_to_json_string;
use crate::interceptors::log_operation;
use crate::models::{CheckInRequest, CheckOutRequest, Lease, LeaseResponse};
use crate::require_login;
use crate::services::LeaseService;

/// 列出所有合同
#[tauri::command]
pub async fn list_leases() -> Result<Vec<Lease>, String> {
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(|e| app_error_to_json_string(e))?;
    let leases = queries::list_leases(&*conn)
        .map_err(app_error_to_json_string)?;

    log_operation("list_leases", None, "leases", 0, true, start.elapsed().as_millis() as u64);
    Ok(leases)
}

/// 获取合同详情
#[tauri::command]
pub async fn get_lease(id: i64) -> Result<Option<Lease>, String> {
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(|e| app_error_to_json_string(e))?;
    let lease = queries::get_lease_by_id(&*conn, id)
        .map_err(app_error_to_json_string)?;

    log_operation("get_lease", None, "lease", id, true, start.elapsed().as_millis() as u64);
    Ok(lease)
}

/// 创建合同
#[tauri::command]
pub async fn create_lease(
    token: String,
    room_id: i64,
    tenant_id: i64,
    start_date: String,
    monthly_rent: i64,
    property_fee: i64,
    deposit: i64,
    contract_number: Option<String>,
    end_date: Option<String>,
) -> Result<i64, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(|e| app_error_to_json_string(e))?;
    let id = queries::create_lease(
        &*conn,
        room_id,
        tenant_id,
        &start_date,
        monthly_rent,
        property_fee,
        deposit,
        contract_number.as_deref(),
        end_date.as_deref(),
    )
    .map_err(app_error_to_json_string)?;

    log_operation("create_lease", None, "lease", id, true, start.elapsed().as_millis() as u64);
    Ok(id)
}

/// 激活合同（草稿 → 生效中）
#[tauri::command]
pub async fn activate_lease(token: String, id: i64) -> Result<LeaseResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let service = LeaseService;
    let result = service.activate(&ctx, id).map_err(app_error_to_json_string)?;

    log_operation("activate_lease", None, "lease", id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 入住
#[tauri::command]
pub async fn check_in(token: String, req: CheckInRequest) -> Result<LeaseResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let service = LeaseService;
    let result = service.check_in(&ctx, &req).map_err(app_error_to_json_string)?;

    log_operation("check_in", req.operator.clone(), "lease", req.lease_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 退房
#[tauri::command]
pub async fn check_out(token: String, req: CheckOutRequest) -> Result<LeaseResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let service = LeaseService;
    let result = service.check_out(&ctx, &req).map_err(app_error_to_json_string)?;

    log_operation("check_out", req.operator.clone(), "lease", req.lease_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 违约标记
#[tauri::command]
pub async fn mark_violation(token: String, lease_id: i64) -> Result<LeaseResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let service = LeaseService;
    let result = service.mark_violation(&ctx, lease_id).map_err(app_error_to_json_string)?;

    log_operation("mark_violation", None, "lease", lease_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 违约恢复
#[tauri::command]
pub async fn recover_from_violation(token: String, lease_id: i64) -> Result<LeaseResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(|e| app_error_to_json_string(e))?;
    let start = std::time::Instant::now();

    let service = LeaseService;
    let result = service.recover_from_violation(&ctx, lease_id).map_err(app_error_to_json_string)?;

    log_operation("recover_from_violation", None, "lease", lease_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}
