//! 合同模型

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, Result};

/// 合同状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStatus {
    Draft,         // 草稿
    Active,        // 生效中
    Violation,     // 违约中
    PendingSettle, // 待结算
    CheckedOut,   // 已退房
    Archived,      // 已归档
    Cancelled,     // 已作废
}

impl LeaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Active => "生效中",
            Self::Violation => "违约中",
            Self::PendingSettle => "待结算",
            Self::CheckedOut => "已退房",
            Self::Archived => "已归档",
            Self::Cancelled => "已作废",
        }
    }

    /// 从字符串转换为枚举（用于从数据库读取状态）
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "草稿" => Ok(Self::Draft),
            "生效中" => Ok(Self::Active),
            "违约中" => Ok(Self::Violation),
            "待结算" => Ok(Self::PendingSettle),
            "已退房" => Ok(Self::CheckedOut),
            "已归档" => Ok(Self::Archived),
            "已作废" => Ok(Self::Cancelled),
            _ => Err(AppError::InvalidStatus(format!("未知合同状态: {}", s))),
        }
    }

    /// 检查是否可以流转到目标状态
    pub fn can_transition_to(&self, next: &LeaseStatus) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Active)
                | (Self::Active, Self::Violation)
                | (Self::Active, Self::PendingSettle)
                | (Self::Violation, Self::PendingSettle)
                | (Self::Violation, Self::Active)      // 违约恢复：违约中 → 生效中
                | (Self::PendingSettle, Self::CheckedOut)
                | (Self::CheckedOut, Self::Archived)
                | (Self::Draft, Self::Cancelled)
        )
    }
}

/// 押金状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepositStatus {
    Unreceived,   // 未收取
    Received,     // 已收取
    Partial,      // 部分收取
    Refunded,     // 退还
    Forfeited,    // 没收
}

impl DepositStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unreceived => "未收取",
            Self::Received => "已收取",
            Self::Partial => "部分收取",
            Self::Refunded => "退还",
            Self::Forfeited => "没收",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "未收取" => Ok(Self::Unreceived),
            "已收取" => Ok(Self::Received),
            "部分收取" => Ok(Self::Partial),
            "退还" => Ok(Self::Refunded),
            "没收" => Ok(Self::Forfeited),
            _ => Err(AppError::InvalidStatus(format!("未知押金状态: {}", s))),
        }
    }

    pub fn can_transition_to(&self, next: &DepositStatus) -> bool {
        matches!(
            (self, next),
            (Self::Unreceived, Self::Partial)
                | (Self::Unreceived, Self::Received)
                | (Self::Partial, Self::Partial)
                | (Self::Partial, Self::Received)
                | (Self::Partial, Self::Refunded)
                | (Self::Partial, Self::Forfeited)
                | (Self::Received, Self::Partial)
                | (Self::Received, Self::Refunded)
                | (Self::Received, Self::Forfeited)
        )
    }

    pub fn from_balance(deposit: i64, balance: i64) -> Self {
        if balance >= deposit && deposit > 0 {
            Self::Received
        } else if balance > 0 {
            Self::Partial
        } else {
            Self::Unreceived
        }
    }
}

/// 合同
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Lease {
    pub id: i64,
    pub room_id: i64,
    pub tenant_id: i64,
    pub contract_number: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub monthly_rent: i64,      // 分
    pub property_fee: i64,      // 分
    pub deposit: i64,           // 分
    pub deposit_received: i64,  // 分
    pub deposit_balance: i64,   // 分
    pub deposit_status: String,
    pub move_in_date: Option<String>,
    pub move_out_date: Option<String>,
    pub termination_reason: Option<String>,
    pub status: String,
    pub status_reason: Option<String>,
    pub notes: Option<String>,
    pub is_deleted: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 入住请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CheckInRequest {
    pub room_id: i64,
    pub tenant_id: i64,
    pub lease_id: i64,
    pub move_in_date: String,
    pub operator: Option<String>,
}

/// 退房请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CheckOutRequest {
    pub lease_id: i64,
    pub room_id: i64,
    pub move_out_date: String,
    pub reason: String,
    pub operator: Option<String>,
}

/// 合同响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LeaseResponse {
    pub success: bool,
    pub lease_id: Option<i64>,
    pub message: Option<String>,
}

impl LeaseResponse {
    pub fn success() -> Self {
        Self {
            success: true,
            lease_id: None,
            message: None,
        }
    }

    pub fn success_with_id(lease_id: i64) -> Self {
        Self {
            success: true,
            lease_id: Some(lease_id),
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            lease_id: None,
            message: Some(message.into()),
        }
    }
}

/// 合同详情（带房间和租客信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LeaseDetail {
    pub id: i64,
    pub room_id: i64,
    pub tenant_id: i64,
    pub room_number: String,
    pub tenant_name: String,
    pub tenant_phone: Option<String>,
    pub tenant_phone2: Option<String>,
    pub contract_number: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub monthly_rent: i64,
    pub property_fee: i64,
    pub deposit: i64,
    pub deposit_received: i64,
    pub deposit_balance: i64,
    pub deposit_status: String,
    pub move_in_date: Option<String>,
    pub move_out_date: Option<String>,
    pub termination_reason: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}
