use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::models::report::{MonthlySummaryListResponse, MonthlySummaryResponse};
use crate::require_login;
use crate::services::report_service::ReportService;

#[tauri::command]
pub async fn generate_monthly_summary(token: String, year_month: String) -> Result<MonthlySummaryResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReportService;
    service.generate_monthly_summary(&ctx, &year_month)
        .map_err(app_error_to_json_string)
}

#[tauri::command]
pub async fn get_summary_report(year_month: String) -> Result<MonthlySummaryResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReportService;
    service.get_summary_report(&ctx, &year_month)
        .map_err(app_error_to_json_string)
}

#[tauri::command]
pub async fn list_summary_reports() -> Result<MonthlySummaryListResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReportService;
    service.list_summary_reports(&ctx)
        .map_err(app_error_to_json_string)
}
