//! 数据导出命令
//!
//! 命令层只负责参数解析和转发，业务逻辑委托给 ExportService

use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::interceptors::log_operation;
use crate::require_login;
use crate::services::ExportService;

/// 导出数据
///
/// type: rooms | tenants | bills | payments | summary
/// year_month: 可选的年月筛选（用于 bills 和 payments）
#[tauri::command]
pub fn export_data(token: String, data_type: String, year_month: Option<String>) -> Result<String, String> {
    let _user = require_login!(token);
    let start = std::time::Instant::now();
    let ctx = get_app_context().map_err(app_error_to_json_string)?;

    let service = ExportService;
    let export_data = service
        .export_data(&ctx, &data_type, year_month.as_deref())
        .map_err(app_error_to_json_string)?;

    log_operation(
        "export_data",
        None,
        "export",
        export_data.record_count as i64,
        true,
        start.elapsed().as_millis() as u64,
    );

    Ok(export_data.to_json_string())
}