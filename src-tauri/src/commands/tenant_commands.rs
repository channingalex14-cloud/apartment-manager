//! 租客命令

use crate::db::{get_app_context, queries};
use crate::errors::app_error_to_json_string;
use crate::interceptors::log_operation;
use crate::models::{CreateTenantRequest, TenantHistory, TenantResponse};
use crate::{require_admin, require_login};

/// 列出所有租客
#[tauri::command]
pub async fn list_tenants() -> Result<Vec<TenantResponse>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let tenants = queries::list_tenants(&*conn)
        .map_err(app_error_to_json_string)?
        .into_iter()
        .map(|t| TenantResponse::from(t))
        .collect();

    log_operation("list_tenants", None, "tenants", 0, true, start.elapsed().as_millis() as u64);
    Ok(tenants)
}

/// 获取租客详情
#[tauri::command]
pub async fn get_tenant(id: i64) -> Result<Option<TenantResponse>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let tenant = queries::get_tenant_by_id(&*conn, id)
        .map_err(app_error_to_json_string)?
        .map(|t| TenantResponse::from(t));

    log_operation("get_tenant", None, "tenant", id, true, start.elapsed().as_millis() as u64);
    Ok(tenant)
}

/// 创建租客
#[tauri::command]
pub async fn create_tenant(token: String, req: CreateTenantRequest) -> Result<i64, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let id = queries::create_tenant(
        &*conn,
        &req.name,
        &req.phone,
    ).map_err(app_error_to_json_string)?;

    log_operation("create_tenant", None, "tenant", id, true, start.elapsed().as_millis() as u64);
    Ok(id)
}

/// 更新租客信息
#[tauri::command]
pub async fn update_tenant(
    token: String,
    id: i64,
    name: Option<String>,
    phone: Option<String>,
    phone2: Option<String>,
    emergency_contact: Option<String>,
    emergency_phone: Option<String>,
) -> Result<bool, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let affected = queries::update_tenant(
        &*conn,
        id,
        name.as_deref(),
        phone.as_deref(),
        phone2.as_deref(),
        emergency_contact.as_deref(),
        emergency_phone.as_deref(),
    ).map_err(app_error_to_json_string)?;

    log_operation("update_tenant", None, "tenant", id, affected, start.elapsed().as_millis() as u64);
    Ok(affected)
}

/// 获取租客历史
#[tauri::command]
pub async fn get_tenant_history(tenant_id: i64) -> Result<Vec<TenantHistory>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let history = queries::get_tenant_history(&*conn, tenant_id)
        .map_err(app_error_to_json_string)?;

    log_operation("get_tenant_history", None, "tenant_history", tenant_id, true, start.elapsed().as_millis() as u64);
    Ok(history)
}

#[tauri::command]
pub async fn delete_tenant(token: String, id: i64) -> Result<bool, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let deleted = queries::delete_tenant(&*conn, id).map_err(app_error_to_json_string)?;

    log_operation("delete_tenant", None, "tenant", id, deleted, start.elapsed().as_millis() as u64);
    Ok(deleted)
}
