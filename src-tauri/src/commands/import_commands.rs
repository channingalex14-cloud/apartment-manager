//! 导入命令
//!
//! 命令层只负责参数解析和转发，业务逻辑委托给 ImportService

use crate::errors::{app_error_to_json_string, AppError};
use crate::require_admin;
use crate::services::import_service::BillImportService;
use crate::services::import_service::{ImportBillRequest, ImportBillResponse};
use std::time::Instant;

/// 导入月度账单
#[tauri::command]
pub async fn import_monthly_bills(token: String, req: ImportBillRequest) -> Result<ImportBillResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();
    let service = BillImportService;

    let result = tauri::async_runtime::spawn_blocking(move || {
        service.import_monthly_bills(&req)
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(AppError::Business("后台任务执行失败，请重试".to_string())) })?
    .map_err(app_error_to_json_string)?;

    tracing::info!("import_monthly_bills 完成，耗时: {:?}", start.elapsed());
    Ok(result)
}