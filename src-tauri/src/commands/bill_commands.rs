//! 账单命令

use crate::db::{create_connection, get_app_context, queries, AppContext};
use crate::errors::{app_error_to_json_string, AppError};
use crate::interceptors::log_operation;
use crate::models::{BillResponse, GenerateBillsRequest, MonthlyBill};
use crate::services::bill_service::{BillDetailResponse, BillListResponse, BillSummary};
use crate::services::BillService;
use crate::{require_admin, require_login};
use tauri::async_runtime;

/// 列出账单（原始全量）
#[tauri::command]
pub async fn list_bills(year_month: Option<String>) -> Result<Vec<MonthlyBill>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let bills = queries::list_bills(&*conn, year_month)
        .map_err(app_error_to_json_string)?;

    log_operation("list_bills", None, "bills", 0, true, start.elapsed().as_millis() as u64);
    Ok(bills)
}

/// 生成月度账单
///
/// 注意：这是耗时操作，使用 spawn_blocking 避免阻塞 UI
/// spawn_blocking 内部创建独立的数据库连接，避免跨线程共享 Connection
#[tauri::command]
pub async fn generate_monthly_bills(token: String, req: GenerateBillsRequest) -> Result<BillResponse, String> {
    let _user = require_login!(token);
    let start = std::time::Instant::now();
    let req_boxed = Box::new(req);
    let operator = req_boxed.operator.clone();

    // 使用 spawn_blocking 避免阻塞 UI
    // 在阻塞线程内创建独立连接，避免跨线程共享 Connection
    let result = async_runtime::spawn_blocking(move || {
        let conn = create_connection()?;
        let ctx = AppContext::from_connection(conn);
        let service = BillService;
        service.generate_monthly_bills(&ctx, &req_boxed)
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(AppError::Business("后台任务执行失败，请重试".to_string())) })?
    .map_err(app_error_to_json_string)?;

    log_operation("generate_monthly_bills", operator, "bills", 0, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 查询账单列表（分页 + 筛选）
#[tauri::command]
pub async fn query_bills(
    year: Option<i32>,
    month: Option<i32>,
    room_id: Option<i64>,
    status: Option<String>,
    page: i32,
    page_size: i32,
) -> Result<BillListResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let status_ref = status.as_deref();
    let service = BillService;
    service
        .query_bills(&ctx, year, month, room_id, status_ref, page, page_size)
        .map_err(app_error_to_json_string)
}

/// 查询账单详情
#[tauri::command]
pub async fn get_bill_detail(bill_id: i64) -> Result<BillDetailResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .get_bill_detail(&ctx, bill_id)
        .map_err(app_error_to_json_string)
}

/// 确认账单全额支付
#[tauri::command]
pub fn confirm_bill_paid(token: String, bill_id: i64) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .confirm_bill_paid(&ctx, bill_id)
        .and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))
        .map_err(app_error_to_json_string)
}

/// 账单部分支付
#[tauri::command]
pub fn partial_pay_bill(token: String, bill_id: i64, amount: i64) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .partial_pay_bill(&ctx, bill_id, amount)
        .and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))
        .map_err(app_error_to_json_string)
}

/// 作废账单
#[tauri::command]
pub fn void_bill(token: String, bill_id: i64) -> Result<String, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .void_bill(&ctx, bill_id)
        .and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))
        .map_err(app_error_to_json_string)
}

/// 归档指定年月的所有账单
#[tauri::command]
pub fn archive_bills(token: String, year_month: String) -> Result<String, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .archive_bills(&ctx, &year_month)
        .and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))
        .map_err(app_error_to_json_string)
}

/// 恢复指定年月的已归档账单
#[tauri::command]
pub fn restore_bills(token: String, year_month: String) -> Result<String, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .restore_bills(&ctx, &year_month)
        .and_then(|r| serde_json::to_string(&r).map_err(AppError::Serialization))
        .map_err(app_error_to_json_string)
}

/// 获取所有已归档的年月列表
#[tauri::command]
pub fn list_archived_months() -> Result<Vec<String>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .list_archived_months(&ctx)
        .map_err(app_error_to_json_string)
}

#[tauri::command]
pub fn get_bill_summary(year_month: Option<String>) -> Result<BillSummary, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = BillService;
    service
        .get_bill_summary(&ctx, year_month.as_deref())
        .map_err(app_error_to_json_string)
}
