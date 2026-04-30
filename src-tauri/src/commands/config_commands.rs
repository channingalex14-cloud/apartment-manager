//! 系统配置命令

use crate::db::{get_app_context, list_configs};
use crate::errors::{app_error_to_json_string, AppError};
use crate::interceptors::log_operation;
use crate::models::{ConfigResponse, SystemConfig};
use crate::require_admin;
use std::time::Instant;

/// 获取所有配置
#[tauri::command]
pub async fn get_config() -> Result<Vec<SystemConfig>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let configs = list_configs(&*conn)
        .map_err(app_error_to_json_string)?;

    log_operation(
        "get_config",
        None,
        "config",
        0,
        true,
        start.elapsed().as_millis() as u64,
    );
    Ok(configs)
}

/// 设置配置（需要 admin 权限）
#[tauri::command]
pub async fn set_config(
    token: String,
    key: String,
    value: String,
) -> Result<ConfigResponse, String> {
    let user = require_admin!(token);

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let affected = conn
        .execute(
            "UPDATE system_config SET config_value = ?, updated_at = datetime('now') WHERE config_key = ?",
            rusqlite::params![value, key],
        )
        .map_err(|e| app_error_to_json_string(AppError::Database(e)))?;

    log_operation(
        "set_config",
        Some(user.id.to_string()),
        "config",
        0,
        affected > 0,
        start.elapsed().as_millis() as u64,
    );

    if affected > 0 {
        Ok(ConfigResponse::success())
    } else {
        Ok(ConfigResponse::error(&format!("配置项不存在: {}", key)))
    }
}
