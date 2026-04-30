//! 统一错误类型
//!
//! 所有业务错误都使用 AppError 枚举
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: Database 变体使用 rusqlite::Error
//! - new-db: Database 变体使用 String

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("业务错误: {0}")]
    Business(String),

    #[error("数据不存在: {entity} {id}")]
    NotFound { entity: String, id: i64 },

    #[error("状态不允许: {0}")]
    InvalidStatus(String),

    #[error("金额错误: {0}")]
    InvalidAmount(String),

    #[error("并发冲突: 数据已被其他窗口修改，请刷新后重试")]
    ConcurrentModification,

    #[error("认证失败: {0}")]
    Authentication(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("输入无效: {0}")]
    InvalidInput(String),

    #[cfg(feature = "legacy-db")]
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[cfg(not(feature = "legacy-db"))]
    #[error("数据库错误: {0}")]
    Database(String),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(not(feature = "legacy-db"))]
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

/// 错误响应结构（JSON 序列化给前端）
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl AppError {
    pub fn business(msg: impl Into<String>) -> Self {
        Self::Business(msg.into())
    }

    pub fn not_found(entity: impl Into<String>, id: i64) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id,
        }
    }

    pub fn invalid_status(msg: impl Into<String>) -> Self {
        Self::InvalidStatus(msg.into())
    }

    pub fn invalid_amount(msg: impl Into<String>) -> Self {
        Self::InvalidAmount(msg.into())
    }

    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied(msg.into())
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    pub fn user_message(&self) -> String {
        match self {
            AppError::Business(msg) => msg.clone(),
            AppError::NotFound { entity, id } => format!("{} {} 不存在", entity, id),
            AppError::InvalidStatus(msg) => msg.clone(),
            AppError::InvalidAmount(msg) => msg.clone(),
            AppError::ConcurrentModification => "数据已被其他窗口修改，请刷新后重试".to_string(),
            AppError::Authentication(msg) => msg.clone(),
            AppError::PermissionDenied(msg) => msg.clone(),
            AppError::InvalidInput(msg) => msg.clone(),
            #[cfg(feature = "legacy-db")]
            AppError::Database(_) => "数据库操作失败".to_string(),
            #[cfg(not(feature = "legacy-db"))]
            AppError::Database(_) => "数据库操作失败".to_string(),
            AppError::Serialization(_) => "数据序列化失败".to_string(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (error_type, message, details) = match self {
            AppError::Business(msg) => ("business".to_string(), msg.clone(), None),
            AppError::NotFound { entity, id } => (
                "not_found".to_string(),
                format!("{} {} 不存在", entity, id),
                Some(serde_json::json!({ "entity": entity, "id": id })),
            ),
            AppError::InvalidStatus(msg) => ("invalid_status".to_string(), msg.clone(), None),
            AppError::InvalidAmount(msg) => ("invalid_amount".to_string(), msg.clone(), None),
            AppError::ConcurrentModification => (
                "concurrent_modification".to_string(),
                "数据已被其他窗口修改，请刷新后重试".to_string(),
                None,
            ),
            AppError::Authentication(msg) => (
                "authentication".to_string(),
                msg.clone(),
                None,
            ),
            AppError::PermissionDenied(msg) => (
                "permission_denied".to_string(),
                msg.clone(),
                None,
            ),
            AppError::InvalidInput(msg) => (
                "invalid_input".to_string(),
                msg.clone(),
                None,
            ),
            #[cfg(feature = "legacy-db")]
            AppError::Database(e) => {
                tracing::error!("数据库错误详情: {}", e);
                (
                    "database".to_string(),
                    "数据库操作失败".to_string(),
                    None,
                )
            }
            #[cfg(not(feature = "legacy-db"))]
            AppError::Database(e) => {
                tracing::error!("数据库错误详情: {}", e);
                (
                    "database".to_string(),
                    "数据库操作失败".to_string(),
                    None,
                )
            }
            AppError::Serialization(e) => {
                tracing::error!("序列化错误详情: {}", e);
                (
                    "serialization".to_string(),
                    "数据序列化失败".to_string(),
                    None,
                )
            }
        };

        let err = ErrorResponse {
            error_type,
            message,
            details,
        };
        err.serialize(serializer)
    }
}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, AppError>;

/// 将 AppError 转为 JSON 字符串，用于 Command 层保留结构化错误
pub fn app_error_to_json_string(err: AppError) -> String {
    serde_json::to_string(&err).unwrap_or_else(|e| {
        // 序列化失败时，保留原始错误信息便于调试
        tracing::error!("AppError 序列化失败: {}, 原始错误: {:?}", e, err);
        let fallback = AppError::business(format!("序列化失败: {}", e));
        serde_json::to_string(&fallback).unwrap_or_else(|_| {
            r#"{"error_type":"business","message":"内部错误"}"#.to_string()
        })
    })
}
