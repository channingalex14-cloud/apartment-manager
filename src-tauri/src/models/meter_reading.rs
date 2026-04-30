//! 水电表读数模型

use serde::{Deserialize, Serialize};

/// 单条抄表录入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MeterReadingRequest {
    pub room_id: i64,
    pub year: i32,
    pub month: i32,
    pub water_reading: i64,
    pub electric_reading: i64,
    pub reading_date: String,
    pub operator: Option<String>,
    /// 是否为换表记录（换表后读数可低于上期）
    pub is_replacement: bool,
}

/// 批量抄表录入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BatchMeterReadingRequest {
    pub readings: Vec<MeterReadingRequest>,
}

/// 抄表录入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MeterReadingResponse {
    pub success: bool,
    pub message: String,
    pub id: Option<i64>,
}