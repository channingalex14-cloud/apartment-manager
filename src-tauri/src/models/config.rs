//! 系统配置模型

use serde::{Deserialize, Serialize};

/// 系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemConfig {
    pub id: i64,
    pub config_key: String,
    pub config_value: Option<String>,
    pub config_type: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

/// 获取配置请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GetConfigRequest {
    pub keys: Vec<String>,
}

/// 设置配置请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetConfigRequest {
    pub key: String,
    pub value: String,
    pub operator: Option<String>,
}

/// 配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub success: bool,
    pub message: Option<String>,
}

impl ConfigResponse {
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
        }
    }
}
