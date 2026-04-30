use crate::errors::{app_error_to_json_string, AppError};
use crate::require_admin;
use crate::services::diagnostic_service::{DiagnosticService, RoomTenantFix, ExcelDiagnostic, DbDiagnostic};
use std::time::Instant;

#[tauri::command]
pub async fn diagnose_excel_file(file_path: String) -> Result<ExcelDiagnostic, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        DiagnosticService::diagnose_excel_file(&file_path)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("后台任务执行失败，请稍后重试".to_string()))
    })?;
    Ok(result)
}

#[tauri::command]
pub async fn diagnose_database() -> Result<DbDiagnostic, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        DiagnosticService::diagnose_database().map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("后台任务执行失败，请稍后重试".to_string()))
    })?
    .map_err(|e| e)?;
    Ok(result)
}

#[tauri::command]
pub async fn diagnose_database_text() -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        DiagnosticService::diagnose_database_text().map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("后台任务执行失败，请稍后重试".to_string()))
    })?
    .map_err(|e| e)?;
    Ok(result)
}

#[tauri::command]
pub async fn diagnose_room_detail(room_number: String) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        DiagnosticService::diagnose_room_detail(&room_number).map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("后台任务执行失败，请稍后重试".to_string()))
    })?
    .map_err(|e| e)?;
    Ok(result)
}

#[tauri::command]
pub async fn diagnose_meter_bill(room_id: i64) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        DiagnosticService::diagnose_meter_bill(room_id).map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("后台任务执行失败，请稍后重试".to_string()))
    })?
    .map_err(|e| e)?;
    Ok(result)
}

#[tauri::command]
pub async fn fix_meter_fees(token: String) -> Result<String, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let service = DiagnosticService;
        service.fix_meter_fees().map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking 任务失败: {:?}", e);
        app_error_to_json_string(AppError::Business("修复任务执行失败，请稍后重试".to_string()))
    })?
    .map_err(|e| e)?;

    crate::interceptors::log_operation(
        "fix_meter_fees",
        None,
        "maintenance",
        0,
        true,
        start.elapsed().as_millis() as u64,
    );

    Ok(result.message)
}

#[tauri::command]
pub async fn fix_management_rooms(token: String, fixes: Vec<(String, String, String)>) -> Result<String, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let fixes: Vec<RoomTenantFix> = fixes.into_iter().map(Into::into).collect();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let service = DiagnosticService;
        service.fix_management_rooms(fixes).map_err(app_error_to_json_string)
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(crate::errors::AppError::Business("修复任务执行失败".to_string())) })?
    .map_err(|e| e)?;

    crate::interceptors::log_operation(
        "fix_management_rooms",
        None,
        "maintenance",
        0,
        true,
        start.elapsed().as_millis() as u64,
    );

    Ok(result.message)
}
