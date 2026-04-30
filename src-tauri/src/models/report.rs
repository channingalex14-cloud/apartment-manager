//! 报表模型

use serde::{Deserialize, Serialize};

/// 月度汇总缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MonthlySummaryCache {
    pub id: i64,
    pub year_month: String,
    pub total_rooms: i64,
    pub rented_count: i64,
    pub new_rented_count: i64,
    pub vacant_count: i64,
    pub violation_count: i64,
    pub staff_count: i64,
    pub management_count: i64,
    pub rent_total: i64,
    pub property_total: i64,
    pub water_total: i64,
    pub electric_total: i64,
    pub management_total: i64,
    pub repair_total: i64,
    pub deposit_total: i64,
    pub previous_balance_total: i64,
    pub actual_paid_total: i64,
    pub occupancy_rate: f64,
    pub generated_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 月度汇总缓存响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MonthlySummaryResponse {
    pub success: bool,
    pub data: Option<MonthlySummaryCache>,
    pub message: Option<String>,
}

/// 月度汇总列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MonthlySummaryListResponse {
    pub success: bool,
    pub data: Vec<MonthlySummaryCache>,
    pub message: Option<String>,
}
