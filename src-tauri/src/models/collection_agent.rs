//! 租务 AI Agent 数据模型

use serde::{Deserialize, Serialize};

/// 催租升级级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationLevel {
    None,    // 无逾期或已处理
    Mild,    // 轻微（1-7天）
    Moderate, // 中度（8-14天）
    Severe,  // 严重（15+天）
}

impl EscalationLevel {
    /// 根据逾期天数推导级别
    pub fn from_overdue_days(days: i64) -> Self {
        if days <= 0 {
            Self::None
        } else if days <= 7 {
            Self::Mild
        } else if days <= 14 {
            Self::Moderate
        } else {
            Self::Severe
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Mild => "轻微",
            Self::Moderate => "中度",
            Self::Severe => "严重",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "轻微" => Self::Mild,
            "中度" => Self::Moderate,
            "严重" => Self::Severe,
            _ => Self::None,
        }
    }

    /// 级别对应的中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Mild => "轻微",
            Self::Moderate => "中度",
            Self::Severe => "严重",
        }
    }

    /// 级别对应的颜色（用于前端）
    pub fn color(&self) -> &'static str {
        match self {
            Self::None => "info",
            Self::Mild => "blue",
            Self::Moderate => "warning",
            Self::Severe => "danger",
        }
    }
}

/// 逾期账单（运行时聚合数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverdueBill {
    pub bill_id: i64,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub room_number: String,
    pub building: String,
    pub tenant_name: Option<String>,
    pub tenant_phone: Option<String>,
    pub year_month: String,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub unpaid_amount: i64,
    pub due_date: Option<String>,
    pub overdue_days: i64,
    pub escalation_level: EscalationLevel,
}

/// 催租消息草稿
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraft {
    pub bill_id: i64,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub room_number: String,
    pub building: String,
    pub tenant_name: Option<String>,
    pub tenant_phone: Option<String>,
    pub year_month: String,
    pub unpaid_amount: i64,
    pub overdue_days: i64,
    pub escalation_level: EscalationLevel,
    pub title: String,
    pub message: String,
}

/// Agent 执行报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionAgentReport {
    /// 扫描的账单数
    pub scanned_count: i32,
    /// 欠租账单数
    pub overdue_count: i32,
    /// 轻微级数
    pub mild_count: i32,
    /// 中度级数
    pub moderate_count: i32,
    /// 严重级数
    pub severe_count: i32,
    /// 创建的提醒数
    pub reminders_created: i32,
    /// 标记违约的房间数
    pub rooms_marked_violation: i32,
    /// 欠款总额（分）
    pub total_unpaid: i64,
    /// 催租消息草稿
    pub drafts: Vec<ReminderDraft>,
    /// 是否实际执行（false=预览模式）
    pub executed: bool,
}

/// Agent 请求参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCollectionAgentRequest {
    /// 是否自动升级严重违约
    pub auto_escalate: bool,
    /// 是否自动标记违约（需配置开关）
    pub auto_mark_violation: bool,
    /// 预览模式，不写数据库
    pub dry_run: bool,
    /// 指定年月，None=全部
    pub year_month: Option<String>,
}

impl Default for RunCollectionAgentRequest {
    fn default() -> Self {
        Self {
            auto_escalate: false,
            auto_mark_violation: false,
            dry_run: true,
            year_month: None,
        }
    }
}
