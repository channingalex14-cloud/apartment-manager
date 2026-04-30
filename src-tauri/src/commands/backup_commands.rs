use crate::db::get_database_path;
use crate::errors::{app_error_to_json_string, AppError};
use crate::interceptors::log_operation;
use crate::require_admin;
use crate::services::backup_service::{BackupService, BackupSettings, BackupInfo, BackupResponse};
use std::path::PathBuf;
use std::time::Instant;

#[tauri::command]
pub fn get_backup_settings() -> Result<BackupSettings, String> {
    let ctx = crate::db::get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    BackupService::get_settings(&*conn).map_err(app_error_to_json_string)
}

#[tauri::command]
pub fn save_backup_settings(token: String, settings: BackupSettings) -> Result<(), String> {
    let _user = require_admin!(token);
    let ctx = crate::db::get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    BackupService::save_settings(&*conn, &settings).map_err(app_error_to_json_string)
}

#[tauri::command]
pub async fn backup_database(token: String) -> Result<BackupResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let db_path = get_database_path().map_err(app_error_to_json_string)?;

    let result = tauri::async_runtime::spawn_blocking(move || {
        BackupService::backup(&db_path)
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(AppError::Business("备份任务执行失败".to_string())) })?
    .map_err(app_error_to_json_string)?;

    log_operation(
        "backup_database",
        None,
        "backup",
        0,
        result.success,
        start.elapsed().as_millis() as u64,
    );

    Ok(result)
}

#[tauri::command]
pub fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let db_path = get_database_path().map_err(app_error_to_json_string)?;
    BackupService::list_backups(&db_path).map_err(app_error_to_json_string)
}

#[tauri::command]
pub async fn restore_backup(token: String, backup_path: String) -> Result<String, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let db_path = get_database_path().map_err(app_error_to_json_string)?;
    let restore_path = PathBuf::from(&backup_path);

    let db_path_clone = db_path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        BackupService::restore(&db_path_clone, &restore_path)
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(AppError::Business("恢复任务执行失败".to_string())) })?
    .map_err(app_error_to_json_string)?;

    log_operation(
        "restore_backup",
        None,
        "backup",
        0,
        true,
        start.elapsed().as_millis() as u64,
    );

    Ok(result)
}

#[tauri::command]
pub fn delete_backup(token: String, backup_path: String) -> Result<(), String> {
    let _user = require_admin!(token);
    let path = PathBuf::from(&backup_path);
    BackupService::delete_backup_file(&path).map_err(app_error_to_json_string)
}
