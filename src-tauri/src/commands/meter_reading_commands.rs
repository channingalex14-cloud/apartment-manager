use crate::db::connection::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::models::meter_reading::{BatchMeterReadingRequest, MeterReadingRequest};
use crate::require_login;
use crate::services::MeterReadingService;

#[tauri::command]
pub fn record_meter_reading(token: String, req: MeterReadingRequest) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = MeterReadingService;
    service
        .record_meter_reading(&ctx, &req)
        .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "{}".to_string()))
        .map_err(app_error_to_json_string)
}

#[tauri::command]
pub fn batch_record_meter_readings(token: String, req: BatchMeterReadingRequest) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = MeterReadingService;
    service
        .batch_record_meter_readings(&ctx, &req)
        .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "{}".to_string()))
        .map_err(app_error_to_json_string)
}
