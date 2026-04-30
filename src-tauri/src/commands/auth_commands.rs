//! 认证命令

use crate::auth::{self, LoginRequest, LoginResponse, UserInfo};
use crate::db::get_app_context;
use crate::errors::{app_error_to_json_string, AppError};
use crate::interceptors::log_operation;
use crate::require_admin;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct UserManagementResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub display_name: Option<String>,
    pub is_active: Option<bool>,
}

/// 登录
#[tauri::command]
pub async fn login(request: LoginRequest) -> Result<LoginResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let response = auth::login(&*conn, &request.username, &request.password)
        .map_err(app_error_to_json_string)?;

    let operator = if response.success {
        response.user.as_ref().map(|u| u.id.to_string())
    } else {
        None
    };

    log_operation(
        "login",
        operator,
        "user",
        0,
        response.success,
        start.elapsed().as_millis() as u64,
    );

    Ok(response)
}

/// 登出
#[tauri::command]
pub async fn logout(token: String) -> Result<bool, String> {
    auth::logout(&token);
    Ok(true)
}

/// 获取当前用户信息
#[tauri::command]
pub async fn get_current_user(token: String) -> Result<Option<UserInfo>, String> {
    let user = auth::get_user_by_token(&token);
    Ok(user.map(|u| UserInfo {
        id: u.id,
        username: u.username,
        role: u.role,
        display_name: u.display_name,
    }))
}

/// 获取用户列表（仅 admin）
#[tauri::command]
pub async fn list_users(token: String) -> Result<Vec<auth::User>, String> {
    let _user = require_admin!(token);

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    auth::list_users(&*conn).map_err(app_error_to_json_string)
}

/// 创建用户（仅 admin）
#[tauri::command]
pub async fn create_user(token: String, req: CreateUserRequest) -> Result<UserManagementResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let id = auth::create_user(
        &*conn,
        &req.username,
        &req.password,
        &req.role,
        req.display_name.as_deref(),
    )
    .map_err(app_error_to_json_string)?;

    log_operation(
        "create_user",
        Some(id.to_string()),
        "user",
        id,
        true,
        start.elapsed().as_millis() as u64,
    );

    Ok(UserManagementResponse {
        success: true,
        message: format!("用户 '{}' 创建成功", req.username),
        user_id: Some(id),
    })
}

/// 更新用户（仅 admin）
#[tauri::command]
pub async fn update_user(
    token: String,
    id: i64,
    req: UpdateUserRequest,
) -> Result<UserManagementResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let updated = auth::update_user(
        &*conn,
        id,
        req.role.as_deref(),
        req.display_name.as_deref(),
        req.is_active,
    )
    .map_err(app_error_to_json_string)?;

    log_operation(
        "update_user",
        Some(id.to_string()),
        "user",
        id,
        updated,
        start.elapsed().as_millis() as u64,
    );

    Ok(UserManagementResponse {
        success: updated,
        message: if updated { "用户更新成功".to_string() } else { "无变更".to_string() },
        user_id: Some(id),
    })
}

/// 重置密码（仅 admin）
#[tauri::command]
pub async fn reset_password(
    token: String,
    id: i64,
    new_password: String,
) -> Result<UserManagementResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let reset = auth::reset_password(&*conn, id, &new_password)
        .map_err(app_error_to_json_string)?;

    log_operation(
        "reset_password",
        Some(id.to_string()),
        "user",
        id,
        reset,
        start.elapsed().as_millis() as u64,
    );

    Ok(UserManagementResponse {
        success: reset,
        message: if reset { "密码重置成功".to_string() } else { "用户不存在".to_string() },
        user_id: Some(id),
    })
}

/// 删除用户（仅 admin）
#[tauri::command]
pub async fn delete_user(token: String, id: i64) -> Result<UserManagementResponse, String> {
    let _user = require_admin!(token);
    let start = Instant::now();

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let deleted = auth::delete_user(&*conn, id).map_err(app_error_to_json_string)?;

    log_operation(
        "delete_user",
        Some(id.to_string()),
        "user",
        id,
        deleted,
        start.elapsed().as_millis() as u64,
    );

    Ok(UserManagementResponse {
        success: deleted,
        message: if deleted { "用户删除成功".to_string() } else { "用户不存在".to_string() },
        user_id: Some(id),
    })
}

/// 检查权限
#[tauri::command]
pub async fn check_permission(
    token: String,
    resource: String,
    permission: String,
    action: String,
) -> Result<bool, String> {
    let user = auth::get_user_by_token(&token)
        .ok_or_else(|| app_error_to_json_string(AppError::Authentication("未登录".to_string())))?;

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;

    let required_action = auth::PermissionAction::from_str(&action)
        .map_err(app_error_to_json_string)?;

    auth::check_permission(&*conn, &user.role, &resource, &permission, &required_action)
        .map_err(app_error_to_json_string)
}
