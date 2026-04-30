//! 押金台账模型

use serde::{Deserialize, Serialize};

/// 交易类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Receive,    // 收取
    PartialReceive, // 部分收取
    Refund,     // 退还
    Deduct,     // 抵扣
    Forfeit,    // 没收
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Receive => "收取",
            Self::PartialReceive => "部分收取",
            Self::Refund => "退还",
            Self::Deduct => "抵扣",
            Self::Forfeit => "没收",
        }
    }
}

/// 押金台账记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DepositLedger {
    pub id: i64,
    pub lease_id: i64,
    pub room_id: i64,
    pub transaction_type: String,
    pub amount: i64,            // 分
    pub balance: i64,            // 分
    pub reference_bill_id: Option<i64>,
    pub reference_payment_id: Option<i64>,
    pub operator: Option<String>,
    pub transaction_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

/// 押金台账响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DepositLedgerResponse {
    pub records: Vec<DepositLedgerItem>,
    pub total_balance: i64, // 分
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DepositLedgerItem {
    pub id: i64,
    pub room_number: String,
    pub tenant_name: Option<String>,
    pub transaction_type: String,
    pub amount_fen: i64,       // 金额（分）
    pub balance_fen: i64,      // 余额（分）
    pub transaction_date: Option<String>,
    pub operator: Option<String>,
    pub notes: Option<String>,
}

/// 押金台账行（数据库查询用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DepositLedgerRow {
    pub id: i64,
    pub lease_id: i64,
    pub room_id: i64,
    pub room_number: String,
    pub tenant_name: Option<String>,
    pub transaction_type: String,
    pub amount: i64,
    pub balance: i64,
    pub reference_bill_id: Option<i64>,
    pub reference_payment_id: Option<i64>,
    pub operator: Option<String>,
    pub transaction_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}
