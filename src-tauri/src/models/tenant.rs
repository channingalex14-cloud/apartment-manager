//! 租客模型

use serde::{Deserialize, Serialize};

/// 租客
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Tenant {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub phone2: Option<String>,
    pub emergency_contact: Option<String>,
    pub emergency_phone: Option<String>,
    pub is_deleted: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 创建租客请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateTenantRequest {
    pub name: String,
    pub phone: String,
    pub phone2: Option<String>,
    pub emergency_contact: Option<String>,
    pub emergency_phone: Option<String>,
}

/// 租客响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TenantResponse {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub phone2: Option<String>,
    pub emergency_contact: Option<String>,
    pub emergency_phone: Option<String>,
}

impl From<Tenant> for TenantResponse {
    fn from(t: Tenant) -> Self {
        Self {
            id: t.id,
            name: t.name,
            phone: t.phone,
            phone2: t.phone2,
            emergency_contact: t.emergency_contact,
            emergency_phone: t.emergency_phone,
        }
    }
}

/// 租客历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TenantHistory {
    pub id: i64,
    pub tenant_id: i64,
    pub event_type: String,
    pub room_id: Option<i64>,
    pub lease_id: Option<i64>,
    pub event_date: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}
