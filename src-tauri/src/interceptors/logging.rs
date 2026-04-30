//! 操作日志拦截器
//!
//! 在 Command 执行后自动记录操作日志

use tracing::info;

/// 记录操作日志
///
/// 在每个 Command 结束时调用，用于审计追踪
///
/// # 参数
/// - `command_name`: 命令名称
/// - `operator`: 操作人（None 表示系统）
/// - `entity_type`: 实体类型（room/tenant/lease/bill/payment）
/// - `entity_id`: 实体 ID
/// - `success`: 是否成功
/// - `duration_ms`: 执行耗时（毫秒）
pub fn log_operation(
    command_name: &str,
    operator: Option<String>,
    entity_type: &str,
    entity_id: i64,
    success: bool,
    duration_ms: u64,
) {
    info!(
        command = command_name,
        operator = operator.as_deref().unwrap_or("system"),
        entity = entity_type,
        entity_id = entity_id,
        success = success,
        duration_ms = duration_ms,
        "[OPERATION]"
    );
}
