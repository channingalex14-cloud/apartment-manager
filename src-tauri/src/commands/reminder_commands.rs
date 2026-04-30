//! 提醒命令

use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::models::reminder::{
    CreateReminderRequest, ReminderListResponse, ReminderResponse,
    UpdateReminderStatusRequest,
};
use crate::{require_admin, require_login};
use crate::services::ReminderService;

/// 创建提醒
#[tauri::command]
pub async fn create_reminder(token: String, req: CreateReminderRequest) -> Result<ReminderResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service
        .create_reminder(&ctx, &req)
        .map_err(app_error_to_json_string)
}

/// 获取提醒列表
#[tauri::command]
pub async fn list_reminders(
    room_id: Option<i64>,
    is_read: Option<bool>,
) -> Result<ReminderListResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service
        .list_reminders(&ctx, room_id, is_read)
        .map_err(app_error_to_json_string)
}

/// 获取待发送的提醒
#[tauri::command]
pub async fn get_pending_reminders() -> Result<ReminderListResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service
        .get_pending_reminders(&ctx)
        .map_err(app_error_to_json_string)
}

/// 更新提醒状态
#[tauri::command]
pub async fn update_reminder_status(
    token: String,
    id: i64,
    req: UpdateReminderStatusRequest,
) -> Result<ReminderResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service
        .update_reminder_status(&ctx, id, &req)
        .map_err(app_error_to_json_string)
}

/// 标记提醒为已发送
#[tauri::command]
pub async fn mark_reminder_sent(token: String, id: i64) -> Result<ReminderResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service.mark_as_sent(&ctx, id).map_err(app_error_to_json_string)
}

/// 标记提醒为已读
#[tauri::command]
pub async fn mark_reminder_read(token: String, id: i64) -> Result<ReminderResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service.mark_as_read(&ctx, id).map_err(app_error_to_json_string)
}

/// 删除提醒
#[tauri::command]
pub async fn delete_reminder(token: String, id: i64) -> Result<ReminderResponse, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = ReminderService;
    service
        .delete_reminder(&ctx, id)
        .map_err(app_error_to_json_string)
}
