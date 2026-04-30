//! 账单模型

use crate::errors::AppError;
use crate::models::RoomStatus;
use serde::{Deserialize, Serialize};

/// 账单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillStatus {
    Pending,   // 待缴费
    Partial,   // 部分支付
    Paid,      // 已支付
    Voided,    // 已作废
}

impl BillStatus {
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "待缴费" => Ok(Self::Pending),
            "部分支付" => Ok(Self::Partial),
            "已支付" => Ok(Self::Paid),
            "已作废" => Ok(Self::Voided),
            _ => Err(AppError::InvalidStatus(format!("未知账单状态: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "待缴费",
            Self::Partial => "部分支付",
            Self::Paid => "已支付",
            Self::Voided => "已作废",
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Paid | Self::Voided)
    }

    /// 根据已付金额推导状态
    pub fn from_paid_amount(total_amount: i64, paid_amount: i64) -> Self {
        if paid_amount <= 0 {
            Self::Pending
        } else if paid_amount >= total_amount {
            Self::Paid
        } else {
            Self::Partial
        }
    }

    pub fn can_transition_to(&self, next: &BillStatus) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Paid)
                | (Self::Pending, Self::Partial)
                | (Self::Pending, Self::Voided)
                | (Self::Partial, Self::Paid)
                | (Self::Partial, Self::Voided)
        )
    }
}

/// 账单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillType {
    Normal,           // 正常
    FirstMonthFree,   // 首月免水电
    HalfMonth,        // 半月结算
    LastMonth,        // 末月结算
    MidMonthCheckout, // 月中退房结算
}

impl BillType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::FirstMonthFree => "首月免水电",
            Self::HalfMonth => "半月结算",
            Self::LastMonth => "末月结算",
            Self::MidMonthCheckout => "月中退房结算",
        }
    }
}

/// 月度账单
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MonthlyBill {
    pub id: i64,
    pub year_month: String,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub lease_start_date: Option<String>,
    pub lease_end_date: Option<String>,
    pub check_in_day: Option<i32>,
    pub check_out_day: Option<i32>,
    pub water_reading_prev: i64,
    pub water_reading_current: i64,
    pub electric_reading_prev: i64,
    pub electric_reading_current: i64,
    pub water_usage: i64,
    pub electric_usage: i64,
    pub water_unit_price: i64,     // 分/方
    pub electric_unit_price: i64,  // 分/度
    pub management_unit_price: i64, // 分/度
    pub rent_fee: i64,
    pub rent_days: i32,
    pub rent_daily_rate: i64,
    pub property_fee: i64,
    pub water_fee: i64,
    pub electric_fee: i64,
    pub management_fee: i64,
    pub repair_fee: i64,
    pub misc_fee: i64,
    pub misc_fee_remark: Option<String>,
    pub deposit_fee: i64,
    pub previous_balance: i64,
    pub actual_paid: i64,
    pub total_amount: i64,
    pub bill_type: String,
    pub room_status: RoomStatus,
    pub status: String,
    pub due_date: Option<String>,
    pub paid_date: Option<String>,
    pub bill_sequence: i32,
    pub is_deleted: bool,
    pub is_archived: bool,  // 是否已归档（归档后默认不显示）
    pub archived_at: Option<String>,  // 归档时间
    pub notes: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 生成账单请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GenerateBillsRequest {
    pub year_month: String,
    pub room_ids: Option<Vec<i64>>, // None 表示全部房间
    pub operator: Option<String>,
    pub misc_fee: Option<i64>,        // 杂费金额（分），默认0
    pub misc_fee_remark: Option<String>, // 杂费备注，如"维修费"
}

/// 账单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BillResponse {
    pub success: bool,
    pub generated_count: i32,
    pub message: Option<String>,
}

impl BillResponse {
    pub fn success(count: i32) -> Self {
        Self {
            success: true,
            generated_count: count,
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            generated_count: 0,
            message: Some(message.into()),
        }
    }
}

/// 单个房间的账单生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoomBillResult {
    pub room_id: i64,
    pub room_number: String,
    pub bill_id: Option<i64>,         // None=跳过
    pub skip_reason: Option<String>,

    // 费用明细（跳过时全为0）
    pub rent: i64,                   // 房租（分）
    pub property_fee: i64,           // 物业费（分）
    pub water_fee: i64,             // 水费（分）
    pub electric_fee: i64,           // 电费（分）
    pub management_fee: i64,        // 管理费（分）
    pub misc_fee: i64,              // 杂费（分）
    pub misc_fee_remark: Option<String>, // 杂费备注
    pub previous_balance: i64,       // 上期欠费（分）
    pub total: i64,                 // 合计（分）
}

impl RoomBillResult {
    /// 创建跳过结果
    pub fn skipped(room_id: i64, room_number: String, reason: &str) -> Self {
        Self {
            room_id,
            room_number,
            bill_id: None,
            skip_reason: Some(reason.to_string()),
            rent: 0,
            property_fee: 0,
            water_fee: 0,
            electric_fee: 0,
            management_fee: 0,
            misc_fee: 0,
            misc_fee_remark: None,
            previous_balance: 0,
            total: 0,
        }
    }

    /// 创建成功结果
    pub fn success(
        room_id: i64,
        room_number: String,
        bill_id: i64,
        rent: i64,
        property_fee: i64,
        water_fee: i64,
        electric_fee: i64,
        management_fee: i64,
        misc_fee: i64,
        misc_fee_remark: Option<String>,
        previous_balance: i64,
        total: i64,
    ) -> Self {
        Self {
            room_id,
            room_number,
            bill_id: Some(bill_id),
            skip_reason: None,
            rent,
            property_fee,
            water_fee,
            electric_fee,
            management_fee,
            misc_fee,
            misc_fee_remark,
            previous_balance,
            total,
        }
    }
}

/// 账单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BillDetails {
    pub rent: i64,                   // 房租（分）
    pub property_fee: i64,           // 物业费（分）
    pub water_fee: i64,             // 水费（分）
    pub electric_fee: i64,           // 电费（分）
    pub management_fee: i64,        // 管理费（分）
    pub misc_fee: i64,              // 杂费（分）
    pub misc_fee_remark: Option<String>, // 杂费备注
    pub previous_balance: i64,       // 上期欠费（分）
    pub total: i64,                 // 合计（分）
}

/// 账单预览（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BillPreview {
    pub room_id: i64,
    pub room_number: String,
    pub room_status: RoomStatus,
    pub water_usage: i64,
    pub electric_usage: i64,
    pub rent_fee_fen: i64,
    pub property_fee_fen: i64,
    pub water_fee_fen: i64,
    pub electric_fee_fen: i64,
    pub management_fee_fen: i64,
    pub total_fee_fen: i64,
}

/// 归档账单请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArchiveBillsRequest {
    pub year_month: String,  // 要归档的年月，如 "2024-01"
}

/// 归档账单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArchiveBillsResponse {
    pub success: bool,
    pub archived_count: i32,
    pub message: Option<String>,
}

impl ArchiveBillsResponse {
    pub fn success(count: i32) -> Self {
        Self {
            success: true,
            archived_count: count,
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            archived_count: 0,
            message: Some(message.into()),
        }
    }
}

/// 恢复归档请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RestoreBillsRequest {
    pub year_month: String,  // 要恢复归档的年月，如 "2024-01"
}
