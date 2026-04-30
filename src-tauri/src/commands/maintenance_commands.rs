use crate::db::{create_connection, vacuum_database};
use crate::errors::{app_error_to_json_string, AppError};
use crate::require_admin;
use std::time::Instant;
use tracing::info;

#[tauri::command]
pub async fn vacuum_database_cmd(token: String) -> Result<String, String> {
    let _user = require_admin!(token);
    let start = Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let conn = create_connection()?;
        vacuum_database(&conn)?;
        Ok::<(), AppError>(())
    })
    .await
    .map_err(|e| { tracing::error!("spawn_blocking 任务失败: {}", e); app_error_to_json_string(AppError::Business("VACUUM 任务执行失败".to_string())) })?;

    result.map_err(app_error_to_json_string)?;

    let elapsed = start.elapsed().as_millis();
    info!("VACUUM 完成，耗时 {}ms", elapsed);

    Ok(format!("数据库压缩完成，耗时 {}ms", elapsed))
}
