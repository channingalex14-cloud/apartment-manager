//! 缴费模型

use serde::{Deserialize, Serialize};

/// 支付方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Wechat,     // 微信
    Alipay,     // 支付宝
    BankCard,   // 银行卡
    Cash,       // 现金
    MerchantCode, // 商家码
    DepositDeduct, // 押金抵扣
    Mixed,      // 混合支付
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wechat => "微信",
            Self::Alipay => "支付宝",
            Self::BankCard => "银行卡",
            Self::Cash => "现金",
            Self::MerchantCode => "商家码",
            Self::DepositDeduct => "押金抵扣",
            Self::Mixed => "混合支付",
        }
    }
}

/// 缴费记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Payment {
    pub id: i64,
    pub bill_id: Option<i64>,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub amount: i64,            // 分
    pub payment_date: Option<String>,
    pub payment_method: Option<String>,
    pub wechat_amount: i64,     // 分
    pub alipay_amount: i64,     // 分
    pub cash_amount: i64,       // 分
    pub bank_amount: i64,       // 分
    pub deposit_deduct_amount: i64, // 分
    pub payer_name: Option<String>,
    pub confirmation_screenshot: Option<String>,
    pub operator: Option<String>,
    pub notes: Option<String>,
    pub is_deleted: bool,
    pub created_at: Option<String>,
}

/// 缴费请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecordPaymentRequest {
    pub bill_id: i64,
    pub amount: i64,            // 分
    pub payment_method: String,
    pub payment_date: String,
    pub payer_name: Option<String>,
    pub wechat_amount: Option<i64>,
    pub alipay_amount: Option<i64>,
    pub cash_amount: Option<i64>,
    pub bank_amount: Option<i64>,
    pub deposit_deduct_amount: Option<i64>,
    pub operator: Option<String>,
}

/// 缴费响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PaymentResponse {
    pub success: bool,
    pub payment_id: Option<i64>,
    pub message: Option<String>,
}

impl PaymentResponse {
    pub fn success(payment_id: i64) -> Self {
        Self {
            success: true,
            payment_id: Some(payment_id),
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            payment_id: None,
            message: Some(message.into()),
        }
    }
}
