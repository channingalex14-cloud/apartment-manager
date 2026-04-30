//! 提醒模型

use serde::{Deserialize, Serialize};

/// 提醒类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Reminder {
    pub id: i64,
    pub reminder_type: String,
    pub room_id: Option<i64>,
    pub lease_id: Option<i64>,
    pub title: String,
    pub message: Option<String>,
    pub scheduled_date: Option<String>,
    pub reminded_at: Option<String>,
    pub is_sent: bool,
    pub is_read: bool,
    pub created_at: Option<String>,
}

/// 创建提醒请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateReminderRequest {
    pub reminder_type: String,
    pub room_id: Option<i64>,
    pub lease_id: Option<i64>,
    pub title: String,
    pub message: Option<String>,
    pub scheduled_date: Option<String>,
}

/// 更新提醒状态请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateReminderStatusRequest {
    pub is_sent: Option<bool>,
    pub is_read: Option<bool>,
}

/// 提醒响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReminderResponse {
    pub success: bool,
    pub reminder_id: Option<i64>,
    pub message: Option<String>,
}

/// 提醒列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReminderListResponse {
    pub success: bool,
    pub data: Vec<Reminder>,
    pub message: Option<String>,
}
