//! 房间模型

use serde::{Deserialize, Serialize};
use std::fmt;

/// 房间状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomStatus {
    Vacant,      // 空房
    Rented,      // 在租
    NewRented,   // 新租
    Staff,       // 员工
    Management,  // 管理
    Violation,   // 违约
    Maintenance, // 维修中
    PendingClean, // 待清洁
}

impl RoomStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vacant => "空房",
            Self::Rented => "在租",
            Self::NewRented => "新租",
            Self::Staff => "员工",
            Self::Management => "管理",
            Self::Violation => "违约",
            Self::Maintenance => "维修中",
            Self::PendingClean => "待清洁",
        }
    }

    /// 从字符串转换为枚举（用于从数据库读取状态）
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "空房" => Ok(Self::Vacant),
            "在租" => Ok(Self::Rented),
            "新租" => Ok(Self::NewRented),
            "员工" => Ok(Self::Staff),
            "管理" => Ok(Self::Management),
            "违约" => Ok(Self::Violation),
            "维修中" => Ok(Self::Maintenance),
            "待清洁" => Ok(Self::PendingClean),
            // 历史兼容：退房 已移除，前端用待清洁替代
            "退房" => Ok(Self::PendingClean),
            _ => Err("未知房间状态"),
        }
    }

    /// 检查该房间状态是否允许入住
    pub fn allows_check_in(&self) -> bool {
        matches!(self, Self::Vacant | Self::PendingClean)
    }

    /// 检查该房间状态是否允许退房
    pub fn allows_check_out(&self) -> bool {
        matches!(self, Self::Rented | Self::NewRented | Self::Violation)
    }

    /// 检查是否可以流转到目标状态（完整状态机，含租约驱动转换）
    pub fn can_transition_to(&self, target: &RoomStatus) -> bool {
        if self == target {
            return true;
        }
        matches!(
            (self, target),
            (Self::Vacant, Self::Rented | Self::NewRented | Self::Maintenance | Self::Management | Self::Staff | Self::PendingClean)
                | (Self::Rented, Self::Violation | Self::PendingClean)
                | (Self::NewRented, Self::Rented | Self::Violation | Self::PendingClean)
                | (Self::Staff, Self::Vacant | Self::Maintenance | Self::Management | Self::PendingClean)
                | (Self::Management, Self::Vacant | Self::Maintenance | Self::Staff | Self::PendingClean)
                | (Self::Violation, Self::Rented | Self::PendingClean)
                | (Self::Maintenance, Self::Vacant | Self::Management | Self::Staff | Self::PendingClean)
                | (Self::PendingClean, Self::Vacant | Self::Maintenance | Self::Management | Self::Staff)
        )
    }

    /// 检查该状态是否允许手动切换到此状态（用于 update_room_status 命令）
    /// 规则：只能手动切换特殊状态（维修中/管理/员工/待清洁/空房），
    /// 有租约的状态（在租/新租/违约）必须通过状态机操作
    pub fn allows_manual_transition_to(&self, target: RoomStatus) -> bool {
        if matches!(self, Self::Rented | Self::NewRented | Self::Violation) {
            return false;
        }
        if matches!(target, Self::Rented | Self::NewRented | Self::Violation) {
            return false;
        }
        matches!(
            (self, target),
            (Self::Vacant, Self::Maintenance | Self::Management | Self::Staff) |
            (Self::Maintenance, Self::Vacant | Self::Management | Self::Staff | Self::PendingClean) |
            (Self::Management, Self::Vacant | Self::Maintenance | Self::Staff | Self::PendingClean) |
            (Self::Staff, Self::Vacant | Self::Maintenance | Self::Management | Self::PendingClean) |
            (Self::PendingClean, Self::Vacant | Self::Maintenance | Self::Management | Self::Staff | Self::NewRented) |
            (Self::Vacant, Self::PendingClean) |
            (Self::PendingClean, Self::Vacant) |
            (_, _) if self == &target
        )
    }
}

impl fmt::Display for RoomStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ========================
// serde: 自定义序列化，确保中文字符串而非 snake_case
// ========================

impl Serialize for RoomStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoomStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(|e| serde::de::Error::custom(e))
    }
}

#[cfg(feature = "legacy-db")]
mod rusqlite_impls {
    use super::RoomStatus;
    use rusqlite::{ToSql, Result as SqlResult, types::{ValueRef, ToSqlOutput, FromSql, FromSqlResult, FromSqlError}};

    impl FromSql for RoomStatus {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            let s = match value {
                ValueRef::Text(s) => std::str::from_utf8(s)
                    .map_err(|_| FromSqlError::InvalidType)?,
                _ => return Err(FromSqlError::InvalidType),
            };
            Self::from_str(s).map_err(|_| FromSqlError::InvalidType)
        }
    }

    impl ToSql for RoomStatus {
        fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::from(self.as_str().to_owned()))
        }
    }
}

/// 房间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Room {
    pub id: i64,
    pub room_number: String,
    pub floor: Option<i32>,
    pub building: String,
    pub room_type: String,     // 房型：单间、一房一厅、二房一厅、三房一厅、商铺
    pub base_rent: i64,       // 分
    pub property_fee: i64,    // 分
    pub deposit: i64,         // 分
    pub status: RoomStatus,
    pub water_meter_current: i64,
    pub electric_meter_current: i64,
    pub is_deleted: bool,
    pub version: i64,         // 乐观锁版本号
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 房间响应（前端展示用，元为单位）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoomResponse {
    pub id: i64,
    pub room_number: String,
    pub floor: Option<i32>,
    pub building: String,
    pub room_type: String,         // 房型
    pub base_rent_fen: i64,
    pub property_fee_fen: i64,
    pub deposit_fen: i64,
    pub status: RoomStatus,
    pub water_meter_current: i64,
    pub electric_meter_current: i64,
    pub tenant_name: Option<String>,
    pub tenant_phone: Option<String>,
    pub lease_id: Option<i64>,
    pub lease_start_date: Option<String>,
    pub lease_end_date: Option<String>,
    pub version: i64,             // 乐观锁版本号，用于并发控制
}

impl From<Room> for RoomResponse {
    fn from(r: Room) -> Self {
        Self {
            id: r.id,
            room_number: r.room_number,
            floor: r.floor,
            building: r.building,
            room_type: r.room_type,
            base_rent_fen: r.base_rent,
            property_fee_fen: r.property_fee,
            deposit_fen: r.deposit,
            status: r.status,
            water_meter_current: r.water_meter_current,
            electric_meter_current: r.electric_meter_current,
            tenant_name: None,
            tenant_phone: None,
            lease_id: None,
            lease_start_date: None,
            lease_end_date: None,
            version: r.version,
        }
    }
}

/// 房间状态变更日志
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoomStatusLog {
    pub id: i64,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub previous_status: Option<RoomStatus>,
    pub new_status: RoomStatus,
    pub trigger_type: String,
    pub tenant_id: Option<i64>,
    pub tenant_name: Option<String>,
    pub change_date: String,
    pub effective_date: Option<String>,
    pub operator: Option<String>,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}
