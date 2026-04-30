//! SQL 查询模板
//!
//! 基础查询封装

// ⚠️ TOCTOU 防护规范（V2.0.3 根因教训）
// ================================================================
// 所有 Service 层的写前读取必须使用 _tx 版本，禁止在事务外读取后事务内写入。
//
// 已知违规模式（❌）：
//   let conn = ctx.get_conn()?;
//   let room = get_room_by_id(&conn, ...)?;    // 事务外读取
//   ctx.transaction(|tx| { tx.execute(...) })?; // 事务内写入
//
// 正确模式（✅）：
//   ctx.transaction(|tx| {
//       let room = get_room_by_id_tx(tx, ...)?; // 事务内读取
//       tx.execute(...);                          // 事务内写入
//   })?;
//
// 新增查询接口时，必须同时提供 &Connection 和 &Transaction 两个版本。
// ================================================================

use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult, Row, Transaction};
use tracing::info;

use crate::errors::Result;
use crate::models::{
    Lease, LeaseDetail, MonthlyBill, Payment, Room, RoomResponse, RoomStatus, SystemConfig, Tenant,
};
use crate::models::LeaseStatus;
use crate::models::BillStatus;

// ========================
// 房间查询
// ========================

/// 从 Row 读取可为 NULL 的 RoomStatus（previous_status 用）
fn get_room_status_nullable(row: &Row, col_name: &str) -> SqliteResult<Option<RoomStatus>> {
    use rusqlite::types::ValueRef;
    let value = row.get_ref(col_name)?;
    match value {
        ValueRef::Null => Ok(None),
        ValueRef::Text(s) => {
            let s_str =
                std::str::from_utf8(s).map_err(|_| rusqlite::Error::InvalidQuery)?;
            RoomStatus::from_str(s_str)
                .map(Some)
                .map_err(|_| rusqlite::Error::InvalidQuery)
        }
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

/// 查询所有房间（带当前租客信息）
pub fn list_rooms(conn: &Connection) -> Result<Vec<RoomResponse>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            r.id, r.room_number, r.floor, r.building, r.room_type,
            r.base_rent, r.property_fee, r.deposit, r.status,
            r.water_meter_current, r.electric_meter_current, r.version,
            t.name as tenant_name, t.phone as tenant_phone,
            l.id as lease_id, l.start_date as lease_start_date, l.end_date as lease_end_date
        FROM rooms r
        LEFT JOIN (
            SELECT id, room_id, tenant_id, start_date, end_date, status, is_deleted
            FROM (
                SELECT id, room_id, tenant_id, start_date, end_date, status, is_deleted,
                       ROW_NUMBER() OVER (PARTITION BY room_id ORDER BY start_date DESC) as rn
                FROM leases
                WHERE status = ? AND is_deleted = 0
            )
            WHERE rn = 1  -- 每个房间只保留最新一条租约
        ) l ON l.room_id = r.id
        LEFT JOIN tenants t ON t.id = l.tenant_id AND t.is_deleted = 0
        WHERE r.is_deleted = 0
        ORDER BY COALESCE(r.floor, 0), r.room_number
        "#,
    )?;

    let rooms: Vec<RoomResponse> = stmt
        .query_map([LeaseStatus::Active.as_str()], |row| {
            Ok(RoomResponse {
                id: row.get("id")?,
                room_number: row.get("room_number")?,
                floor: row.get("floor")?,
                building: row.get("building")?,
                room_type: row.get("room_type")?,
                base_rent_fen: row.get::<_, i64>("base_rent")?,
                property_fee_fen: row.get::<_, i64>("property_fee")?,
                deposit_fen: row.get::<_, i64>("deposit")?,
                status: row.get("status")?,
                water_meter_current: row.get("water_meter_current")?,
                electric_meter_current: row.get("electric_meter_current")?,
                tenant_name: row.get("tenant_name")?,
                tenant_phone: row.get("tenant_phone")?,
                lease_id: row.get("lease_id")?,
                lease_start_date: row.get("lease_start_date")?,
                lease_end_date: row.get("lease_end_date")?,
                version: row.get::<_, i64>("version").unwrap_or(0),
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    info!("[QUERY] list_rooms 返回 {} 条记录", rooms.len());
    Ok(rooms)
}

/// 根据 ID 查询房间
pub fn get_room_by_id(conn: &Connection, id: i64) -> Result<Option<Room>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM rooms WHERE id = ? AND is_deleted = 0",
    )?;

    let room = stmt
        .query_row([id], |row| {
            Ok(Room {
                id: row.get("id")?,
                room_number: row.get("room_number")?,
                floor: row.get("floor")?,
                building: row.get("building")?,
                room_type: row.get("room_type")?,
                base_rent: row.get("base_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                status: row.get("status")?,
                water_meter_current: row.get("water_meter_current")?,
                electric_meter_current: row.get("electric_meter_current")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                version: row.get::<_, i64>("version").unwrap_or(0), // 乐观锁版本号
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(room)
}

/// 根据 ID 查询房间（事务版本，用于 TOCTOU 防护）
pub fn get_room_by_id_tx(tx: &Transaction, id: i64) -> Result<Option<Room>> {
    let mut stmt = tx.prepare(
        "SELECT * FROM rooms WHERE id = ? AND is_deleted = 0",
    )?;

    let room = stmt
        .query_row([id], |row| {
            Ok(Room {
                id: row.get("id")?,
                room_number: row.get("room_number")?,
                floor: row.get("floor")?,
                building: row.get("building")?,
                room_type: row.get("room_type")?,
                base_rent: row.get("base_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                status: row.get("status")?,
                water_meter_current: row.get("water_meter_current")?,
                electric_meter_current: row.get("electric_meter_current")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                version: row.get::<_, i64>("version").unwrap_or(0), // 乐观锁版本号
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(room)
}

/// 根据 ID 查询房间（带租客信息，用于 get_room 命令）
pub fn get_room_response_by_id(conn: &Connection, id: i64) -> Result<Option<RoomResponse>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            r.id, r.room_number, r.floor, r.building, r.room_type,
            r.base_rent, r.property_fee, r.deposit, r.status,
            r.water_meter_current, r.electric_meter_current, r.version,
            t.name as tenant_name, t.phone as tenant_phone,
            l.id as lease_id, l.start_date as lease_start_date, l.end_date as lease_end_date
        FROM rooms r
        LEFT JOIN (
            SELECT id, room_id, tenant_id, start_date, end_date, status, is_deleted
            FROM leases
            WHERE status = ? AND is_deleted = 0
        ) l ON l.room_id = r.id
        LEFT JOIN tenants t ON t.id = l.tenant_id AND t.is_deleted = 0
        WHERE r.id = ? AND r.is_deleted = 0
        "#,
    )?;

    let room = stmt
        .query_row(params![LeaseStatus::Active.as_str(), id], |row| {
            Ok(RoomResponse {
                id: row.get("id")?,
                room_number: row.get("room_number")?,
                floor: row.get("floor")?,
                building: row.get("building")?,
                room_type: row.get("room_type")?,
                base_rent_fen: row.get::<_, i64>("base_rent")?,
                property_fee_fen: row.get::<_, i64>("property_fee")?,
                deposit_fen: row.get::<_, i64>("deposit")?,
                status: row.get("status")?,
                water_meter_current: row.get("water_meter_current")?,
                electric_meter_current: row.get("electric_meter_current")?,
                tenant_name: row.get("tenant_name")?,
                tenant_phone: row.get("tenant_phone")?,
                lease_id: row.get("lease_id")?,
                lease_start_date: row.get("lease_start_date")?,
                lease_end_date: row.get("lease_end_date")?,
                version: row.get::<_, i64>("version").unwrap_or(0),
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(room)
}

/// ========================
//租客查询
/// ========================

/// 查询所有租客
pub fn list_tenants(conn: &Connection) -> Result<Vec<Tenant>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tenants WHERE is_deleted = 0 ORDER BY name",
    )?;

    let tenants = stmt
        .query_map([], |row| {
            Ok(Tenant {
                id: row.get("id")?,
                name: row.get("name")?,
                phone: row.get("phone")?,
                phone2: row.get("phone2")?,
                emergency_contact: row.get("emergency_contact")?,
                emergency_phone: row.get("emergency_phone")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(tenants)
}

/// 根据 ID 查询租客
pub fn get_tenant_by_id(conn: &Connection, id: i64) -> Result<Option<Tenant>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM tenants WHERE id = ? AND is_deleted = 0",
    )?;

    let tenant = stmt
        .query_row([id], |row| {
            Ok(Tenant {
                id: row.get("id")?,
                name: row.get("name")?,
                phone: row.get("phone")?,
                phone2: row.get("phone2")?,
                emergency_contact: row.get("emergency_contact")?,
                emergency_phone: row.get("emergency_phone")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(tenant)
}

/// 创建租客（简化版）
pub fn create_tenant(conn: &Connection, name: &str, phone: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO tenants (name, phone) VALUES (?, ?)",
        params![name, phone],
    )
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(conn.last_insert_rowid())
}

/// 创建租客（完整版）
pub fn create_tenant_full(
    conn: &Connection,
    name: &str,
    phone: &str,
    phone2: Option<&str>,
    emergency_contact: Option<&str>,
    emergency_phone: Option<&str>,
) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO tenants (name, phone, phone2, emergency_contact, emergency_phone)
        VALUES (?, ?, ?, ?, ?)
        "#,
        params![name, phone, phone2, emergency_contact, emergency_phone],
    )
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(conn.last_insert_rowid())
}

/// 更新租客
pub fn update_tenant(
    conn: &Connection,
    id: i64,
    name: Option<&str>,
    phone: Option<&str>,
    phone2: Option<&str>,
    emergency_contact: Option<&str>,
    emergency_phone: Option<&str>,
) -> Result<bool> {
    let mut updates = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(v) = name {
        updates.push("name = ?");
        params_vec.push(Box::new(v.to_string()));
    }
    if let Some(v) = phone {
        updates.push("phone = ?");
        params_vec.push(Box::new(v.to_string()));
    }
    if phone2.is_some() {
        updates.push("phone2 = ?");
        params_vec.push(Box::new(phone2.unwrap_or("").to_string()));
    }
    if emergency_contact.is_some() {
        updates.push("emergency_contact = ?");
        params_vec.push(Box::new(emergency_contact.unwrap_or("").to_string()));
    }
    if emergency_phone.is_some() {
        updates.push("emergency_phone = ?");
        params_vec.push(Box::new(emergency_phone.unwrap_or("").to_string()));
    }

    if updates.is_empty() {
        return Ok(false);
    }

    updates.push("updated_at = datetime('now')");
    params_vec.push(Box::new(id));

    // 安全说明：
    // - 列名（updates）来源于硬编码白名单（name/phone/phone2/...），非用户输入
    // - 列值通过 ? 参数占位符绑定（params_vec），非字符串拼接
    // - 即使 format! 拼接了列名，列名本身也是白名单内的硬编码值，无注入风险
    let sql = format!("UPDATE tenants SET {} WHERE id = ?", updates.join(", "));
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let affected = conn.execute(&sql, params_refs.as_slice())
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(affected > 0)
}

/// 软删除租客
pub fn delete_tenant(conn: &Connection, id: i64) -> Result<bool> {
    let affected = conn
        .execute(
            "UPDATE tenants SET is_deleted = 1, updated_at = datetime('now') WHERE id = ? AND is_deleted = 0",
            [id],
        )
        .map_err(|e| crate::errors::AppError::Database(e))?;
    Ok(affected > 0)
}

/// ========================
//合同查询
/// ========================

/// 查询所有合同
pub fn list_leases(conn: &Connection) -> Result<Vec<Lease>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT * FROM leases
        WHERE is_deleted = 0
        ORDER BY created_at DESC
        "#,
    )?;

    let leases = stmt
        .query_map([], |row| {
            Ok(Lease {
                id: row.get("id")?,
                room_id: row.get("room_id")?,
                tenant_id: row.get("tenant_id")?,
                contract_number: row.get("contract_number")?,
                start_date: row.get("start_date")?,
                end_date: row.get("end_date")?,
                monthly_rent: row.get("monthly_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                deposit_received: row.get("deposit_received")?,
                deposit_balance: row.get("deposit_balance")?,
                deposit_status: row.get("deposit_status")?,
                move_in_date: row.get("move_in_date")?,
                move_out_date: row.get("move_out_date")?,
                termination_reason: row.get("termination_reason")?,
                status: row.get("status")?,
                status_reason: row.get("status_reason")?,
                notes: row.get("notes")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(leases)
}

/// 根据 ID 查询合同
pub fn get_lease_by_id(conn: &Connection, id: i64) -> Result<Option<Lease>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM leases WHERE id = ? AND is_deleted = 0",
    )?;

    let lease = stmt
        .query_row([id], |row| {
            Ok(Lease {
                id: row.get("id")?,
                room_id: row.get("room_id")?,
                tenant_id: row.get("tenant_id")?,
                contract_number: row.get("contract_number")?,
                start_date: row.get("start_date")?,
                end_date: row.get("end_date")?,
                monthly_rent: row.get("monthly_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                deposit_received: row.get("deposit_received")?,
                deposit_balance: row.get("deposit_balance")?,
                deposit_status: row.get("deposit_status")?,
                move_in_date: row.get("move_in_date")?,
                move_out_date: row.get("move_out_date")?,
                termination_reason: row.get("termination_reason")?,
                status: row.get("status")?,
                status_reason: row.get("status_reason")?,
                notes: row.get("notes")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(lease)
}

/// 根据 ID 查询合同（事务版本，用于 TOCTOU 防护）
pub fn get_lease_by_id_tx(tx: &Transaction, id: i64) -> Result<Option<Lease>> {
    let mut stmt = tx.prepare(
        "SELECT * FROM leases WHERE id = ? AND is_deleted = 0",
    )?;

    let lease = stmt
        .query_row([id], |row| {
            Ok(Lease {
                id: row.get("id")?,
                room_id: row.get("room_id")?,
                tenant_id: row.get("tenant_id")?,
                contract_number: row.get("contract_number")?,
                start_date: row.get("start_date")?,
                end_date: row.get("end_date")?,
                monthly_rent: row.get("monthly_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                deposit_received: row.get("deposit_received")?,
                deposit_balance: row.get("deposit_balance")?,
                deposit_status: row.get("deposit_status")?,
                move_in_date: row.get("move_in_date")?,
                move_out_date: row.get("move_out_date")?,
                termination_reason: row.get("termination_reason")?,
                status: row.get("status")?,
                status_reason: row.get("status_reason")?,
                notes: row.get("notes")?,
                is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(lease)
}

/// 创建合同
pub fn create_lease(
    conn: &Connection,
    room_id: i64,
    tenant_id: i64,
    start_date: &str,
    monthly_rent: i64,
    property_fee: i64,
    deposit: i64,
    contract_number: Option<&str>,
    end_date: Option<&str>,
) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO leases
        (room_id, tenant_id, contract_number, start_date, end_date,
         monthly_rent, property_fee, deposit, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![room_id, tenant_id, contract_number, start_date, end_date,
                 monthly_rent, property_fee, deposit, LeaseStatus::Draft.as_str()],
    )
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(conn.last_insert_rowid())
}

/// 更新合同状态（带前置状态校验）
pub fn update_lease_status(
    conn: &Connection,
    id: i64,
    status: &str,
    expected_current_status: Option<&str>,
) -> Result<bool> {
    let affected = if let Some(expected) = expected_current_status {
        conn.execute(
            "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ? AND status = ?",
            params![status, id, expected],
        )
    } else {
        conn.execute(
            "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
            params![status, id],
        )
    }
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(affected > 0)
}

/// 合同生效（草稿 → 生效中）
pub fn activate_lease(
    conn: &Connection,
    id: i64,
) -> Result<bool> {
    let affected = conn.execute(
        r#"
        UPDATE leases SET
            status = ?,
            updated_at = datetime('now')
        WHERE id = ? AND status = ?
        "#,
        params![LeaseStatus::Active.as_str(), id, LeaseStatus::Draft.as_str()],
    )
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(affected > 0)
}

/// ========================
//账单查询
/// ========================

/// 查询账单列表
pub fn list_bills(conn: &Connection, year_month: Option<String>) -> Result<Vec<MonthlyBill>> {
    let sql = match &year_month {
        Some(_) => "SELECT * FROM monthly_bills WHERE is_deleted = 0 AND year_month = ? ORDER BY room_id",
        None => "SELECT * FROM monthly_bills WHERE is_deleted = 0 ORDER BY year_month DESC, room_id",
    };

    let mut stmt = conn.prepare(sql)?;

    let bills = match year_month {
        Some(ym) => stmt.query_map([ym], map_bill_row)?,
        None => stmt.query_map([], map_bill_row)?,
    }
    .collect::<SqliteResult<Vec<_>>>()
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(bills)
}

fn map_bill_row(row: &Row) -> SqliteResult<MonthlyBill> {
    Ok(MonthlyBill {
        id: row.get("id")?,
        year_month: row.get("year_month")?,
        room_id: row.get("room_id")?,
        lease_id: row.get("lease_id")?,
        lease_start_date: row.get("lease_start_date")?,
        lease_end_date: row.get("lease_end_date")?,
        check_in_day: row.get("check_in_day")?,
        check_out_day: row.get("check_out_day")?,
        water_reading_prev: row.get("water_reading_prev")?,
        water_reading_current: row.get("water_reading_current")?,
        electric_reading_prev: row.get("electric_reading_prev")?,
        electric_reading_current: row.get("electric_reading_current")?,
        water_usage: row.get("water_usage")?,
        electric_usage: row.get("electric_usage")?,
        water_unit_price: row.get("water_unit_price")?,
        electric_unit_price: row.get("electric_unit_price")?,
        management_unit_price: row.get("management_unit_price")?,
        rent_fee: row.get("rent_fee")?,
        rent_days: row.get("rent_days")?,
        rent_daily_rate: row.get("rent_daily_rate")?,
        property_fee: row.get("property_fee")?,
        water_fee: row.get("water_fee")?,
        electric_fee: row.get("electric_fee")?,
        management_fee: row.get("management_fee")?,
        repair_fee: row.get("repair_fee")?,
        misc_fee: row.get::<_, i64>("misc_fee").unwrap_or(0),
        misc_fee_remark: row.get("misc_fee_remark").ok(),
        deposit_fee: row.get("deposit_fee")?,
        previous_balance: row.get("previous_balance")?,
        actual_paid: row.get("actual_paid")?,
        total_amount: row.get("total_amount")?,
        bill_type: row.get("bill_type")?,
        room_status: row.get("room_status")?,
        status: row.get("status")?,
        due_date: row.get("due_date")?,
        paid_date: row.get("paid_date")?,
        bill_sequence: row.get("bill_sequence")?,
        is_deleted: row.get::<_, i32>("is_deleted")? != 0,
        is_archived: row.get::<_, i32>("is_archived").unwrap_or(0) != 0,  // 归档标志
        archived_at: row.get("archived_at").ok(),  // 归档时间
        notes: row.get("notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 事务版本：查询某房间某月是否已有账单（幂等检查用）
pub fn get_bill_by_room_month_tx(
    tx: &Transaction,
    room_id: i64,
    year_month: &str,
) -> Result<Option<MonthlyBill>> {
    let mut stmt = tx.prepare(
        "SELECT * FROM monthly_bills \
         WHERE room_id = ?1 AND year_month = ?2 AND is_deleted = 0 \
         ORDER BY bill_sequence DESC LIMIT 1",
    )?;

    let result = stmt
        .query_row(params![room_id, year_month], map_bill_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(result)
}

/// 获取某房间最新一条月度账单（含完整水电读数）
pub fn get_latest_bill_for_room(conn: &Connection, room_id: i64) -> Result<Option<MonthlyBill>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM monthly_bills \
         WHERE room_id = ?1 AND is_deleted = 0 \
         ORDER BY year_month DESC, bill_sequence DESC LIMIT 1",
    )?;
    let result = stmt
        .query_row(params![room_id], map_bill_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;
    Ok(result)
}

// ========================
// 账单列表查询
// ========================

/// 账单列表行（精简版，用于分页列表，含房间+租客信息）
#[derive(Debug)]
pub struct BillRow {
    pub id: i64,
    pub room_id: i64,
    pub year_month: String,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub status: String,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    // 关联信息（列表展示用）
    pub room_number: String,
    pub building: String,
    pub tenant_name: Option<String>,
}

impl BillRow {
    fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            year_month: row.get("year_month")?,
            total_amount: row.get("total_amount")?,
            actual_paid: row.get("actual_paid")?,
            status: row.get("status")?,
            due_date: row.get("due_date")?,
            created_at: row.get("created_at")?,
            room_number: row.get("room_number")?,
            building: row.get("building")?,
            tenant_name: row.get("tenant_name")?,
        })
    }
}

/// 账单详情行（包含房间和租客信息）
#[derive(Debug)]
pub struct BillDetailRow {
    pub id: i64,
    pub room_id: i64,
    pub lease_id: Option<i64>,
    pub year_month: String,
    pub rent_fee: i64,
    pub property_fee: i64,
    pub water_fee: i64,
    pub electric_fee: i64,
    pub management_fee: i64,
    pub misc_fee: i64,
    pub misc_fee_remark: Option<String>,
    pub previous_balance: i64,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub status: String,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub water_reading_prev: i64,
    pub water_reading_current: i64,
    pub electric_reading_prev: i64,
    pub electric_reading_current: i64,
    pub repair_fee: i64,
    pub room_number: String,
    pub building: String,
    pub tenant_name: Option<String>,
    pub tenant_phone: Option<String>,
}

impl BillDetailRow {
    fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            lease_id: row.get("lease_id")?,
            year_month: row.get("year_month")?,
            rent_fee: row.get("rent_fee")?,
            property_fee: row.get("property_fee")?,
            water_fee: row.get("water_fee")?,
            electric_fee: row.get("electric_fee")?,
            management_fee: row.get("management_fee")?,
            repair_fee: row.get::<_, i64>("repair_fee").unwrap_or(0),
            misc_fee: row.get::<_, i64>("misc_fee").unwrap_or(0),
            misc_fee_remark: row.get("misc_fee_remark").ok(),
            previous_balance: row.get("previous_balance")?,
            total_amount: row.get("total_amount")?,
            actual_paid: row.get("actual_paid")?,
            status: row.get("status")?,
            due_date: row.get("due_date")?,
            created_at: row.get("created_at")?,
            water_reading_prev: row.get("water_reading_prev")?,
            water_reading_current: row.get("water_reading_current")?,
            electric_reading_prev: row.get("electric_reading_prev")?,
            electric_reading_current: row.get("electric_reading_current")?,
            room_number: row.get("room_number")?,
            building: row.get("building")?,
            tenant_name: row.get("tenant_name")?,
            tenant_phone: row.get("tenant_phone")?,
        })
    }
}

/// 事务版本：分页查询账单列表（支持筛选）
///
/// year/month 传 None 表示不筛选；传 Some 则拼接 WHERE year_month = 'YYYY-MM'
pub fn query_bills_tx(
    tx: &Transaction,
    year: Option<i32>,
    month: Option<i32>,
    room_id: Option<i64>,
    status: Option<&str>,
    page: i32,
    page_size: i32,
) -> Result<(Vec<BillRow>, i32)> {
    // 构建 WHERE 子句（year/month 是受代码控制的 i32/i64，格式化安全）
    let mut where_clauses = vec!["mb.is_deleted = 0".to_string()];

    if let Some(y) = year {
        if let Some(m) = month {
            where_clauses.push(format!("mb.year_month = '{:04}-{:02}'", y, m));
        } else {
            where_clauses.push(format!("mb.year_month LIKE '{:04}-%'", y));
        }
    }
    let where_sql = where_clauses.join(" AND ");

    // 构建参数列表（room_id 和 status 用参数绑定防注入）
    let (count, bills) = if room_id.is_some() || status.is_some() {
        let mut where_with_params = where_clauses.clone();
        let mut param_room_id = None;
        let mut param_status: Option<String> = None;

        if let Some(r) = room_id {
            where_with_params.push("mb.room_id = ?".to_string());
            param_room_id = Some(r);
        }
        if let Some(s) = status {
            where_with_params.push("mb.status = ?".to_string());
            param_status = Some(s.to_string());
        }
        let where_sql_with_params = where_with_params.join(" AND ");

        // 总数
        let sql_count = format!("SELECT COUNT(*) FROM monthly_bills mb WHERE {}", where_sql_with_params);
        let count: i32 = match (param_room_id.clone(), param_status.clone()) {
            (Some(rid), Some(st)) => {
                let mut stmt_count = tx.prepare(&sql_count).map_err(|e| crate::errors::AppError::Database(e))?;
                stmt_count.query_row(params![rid, st], |row| row.get(0))
                    .map_err(|e| crate::errors::AppError::Database(e))?
            }
            (Some(rid), None) => {
                let mut stmt_count = tx.prepare(&sql_count).map_err(|e| crate::errors::AppError::Database(e))?;
                stmt_count.query_row(params![rid], |row| row.get(0))
                    .map_err(|e| crate::errors::AppError::Database(e))?
            }
            (None, Some(st)) => {
                let mut stmt_count = tx.prepare(&sql_count).map_err(|e| crate::errors::AppError::Database(e))?;
                stmt_count.query_row(params![st], |row| row.get(0))
                    .map_err(|e| crate::errors::AppError::Database(e))?
            }
            (None, None) => unreachable!(),
        };

        // 分页数据
        let offset = (page - 1) * page_size;
        let sql = format!(
            "SELECT mb.id, mb.room_id, mb.year_month, \
                    mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at, \
                    r.room_number, r.building, \
                    t.name as tenant_name \
             FROM monthly_bills mb \
             LEFT JOIN rooms r ON mb.room_id = r.id \
             LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0 \
             LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0 \
             WHERE {} \
             ORDER BY mb.year_month DESC, mb.room_id \
             LIMIT ? OFFSET ?",
            where_sql_with_params
        );
        let mut stmt = tx.prepare(&sql).map_err(|e| crate::errors::AppError::Database(e))?;
        let bills = match (param_room_id.clone(), param_status.clone()) {
            (Some(rid), Some(st)) => {
                stmt.query_map(params![rid, st, page_size, offset], BillRow::from_row)
            }
            (Some(rid), None) => {
                stmt.query_map(params![rid, page_size, offset], BillRow::from_row)
            }
            (None, Some(st)) => {
                stmt.query_map(params![st, page_size, offset], BillRow::from_row)
            }
            (None, None) => unreachable!(),
        }
            .map_err(|e| crate::errors::AppError::Database(e))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| crate::errors::AppError::Database(e))?;

        (count, bills)
    } else {
        // 无需参数绑定的情况（仅有 year/month 筛选）
        let sql_count = format!("SELECT COUNT(*) FROM monthly_bills mb WHERE {}", where_sql);
        let mut stmt_count = tx.prepare(&sql_count).map_err(|e| crate::errors::AppError::Database(e))?;
        let count: i32 = stmt_count
            .query_row([], |row| row.get(0))
            .map_err(|e| crate::errors::AppError::Database(e))?;

        let offset = (page - 1) * page_size;
        let sql = format!(
            "SELECT mb.id, mb.room_id, mb.year_month, \
                    mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at, \
                    r.room_number, r.building, \
                    t.name as tenant_name \
             FROM monthly_bills mb \
             LEFT JOIN rooms r ON mb.room_id = r.id \
             LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0 \
             LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0 \
             WHERE {} \
             ORDER BY mb.year_month DESC, mb.room_id \
             LIMIT ? OFFSET ?",
            where_sql
        );
        let mut stmt = tx.prepare(&sql).map_err(|e| crate::errors::AppError::Database(e))?;
        let bills = stmt
            .query_map(params![page_size, offset], BillRow::from_row)
            .map_err(|e| crate::errors::AppError::Database(e))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| crate::errors::AppError::Database(e))?;

        (count, bills)
    };

    Ok((bills, count))
}

/// 账单行（操作前校验用）
#[derive(Debug)]
pub struct BillForAction {
    pub id: i64,
    pub room_id: i64,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub status: String,
}

impl BillForAction {
    fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            total_amount: row.get("total_amount")?,
            actual_paid: row.get("actual_paid")?,
            status: row.get("status")?,
        })
    }
}

/// 事务版本：查询账单（操作前读取）
pub fn get_bill_by_id_tx(tx: &Transaction, bill_id: i64) -> Result<Option<BillForAction>> {
    let mut stmt = tx.prepare(
        "SELECT id, room_id, total_amount, actual_paid, status \
         FROM monthly_bills WHERE id = ?1 AND is_deleted = 0",
    )?;
    let result = stmt
        .query_row([bill_id], BillForAction::from_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;
    Ok(result)
}

/// 事务版本：查询单张账单详情
pub fn get_bill_detail_tx(
    tx: &Transaction,
    bill_id: i64,
) -> Result<Option<BillDetailRow>> {
    let sql = r#"
        SELECT mb.id, mb.room_id, mb.lease_id, mb.year_month,
               mb.rent_fee, mb.property_fee, mb.water_fee, mb.electric_fee,
               mb.management_fee, mb.repair_fee, mb.misc_fee, mb.misc_fee_remark,
               mb.previous_balance,
               mb.total_amount, mb.actual_paid, mb.status, mb.due_date, mb.created_at,
               mb.water_reading_prev, mb.water_reading_current,
               mb.electric_reading_prev, mb.electric_reading_current,
               r.room_number, r.building,
               t.name as tenant_name, t.phone as tenant_phone
        FROM monthly_bills mb
        LEFT JOIN rooms r ON mb.room_id = r.id
        LEFT JOIN leases l ON mb.lease_id = l.id AND l.is_deleted = 0
        LEFT JOIN tenants t ON l.tenant_id = t.id AND t.is_deleted = 0
        WHERE mb.id = ?1 AND mb.is_deleted = 0
        LIMIT 1
    "#;

    let mut stmt = tx.prepare(sql).map_err(|e| crate::errors::AppError::Database(e))?;
    let result = stmt
        .query_row(params![bill_id], BillDetailRow::from_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(result)
}

/// ========================
//账单归档查询
/// ========================

/// 归档指定年月的所有账单（软删除）
/// 返回归档的账单数量
pub fn archive_bills_by_month_tx(tx: &Transaction, year_month: &str) -> Result<i32> {
    let affected = tx.execute(
        r#"
        UPDATE monthly_bills
        SET is_archived = 1, archived_at = datetime('now')
        WHERE year_month = ? AND is_deleted = 0 AND is_archived = 0
        "#,
        [year_month],
    ).map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(affected as i32)
}

/// 恢复指定年月的已归档账单
/// 返回恢复的账单数量
pub fn restore_bills_by_month_tx(tx: &Transaction, year_month: &str) -> Result<i32> {
    let affected = tx.execute(
        r#"
        UPDATE monthly_bills
        SET is_archived = 0, archived_at = NULL
        WHERE year_month = ? AND is_deleted = 0 AND is_archived = 1
        "#,
        [year_month],
    ).map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(affected as i32)
}

/// 获取指定年月的已归档账单数量
pub fn get_archived_bills_count_tx(tx: &Transaction, year_month: &str) -> Result<i32> {
    let count: i32 = tx
        .query_row(
            "SELECT COUNT(*) FROM monthly_bills WHERE year_month = ? AND is_archived = 1 AND is_deleted = 0",
            [year_month],
            |row| row.get(0),
        )
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(count)
}

/// 获取所有已归档的年月列表
pub fn list_archived_year_months(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT year_month FROM monthly_bills WHERE is_archived = 1 AND is_deleted = 0 ORDER BY year_month DESC",
    )?;

    let months = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| crate::errors::AppError::Database(e))?
        .collect::<SqliteResult<Vec<String>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(months)
}

/// ========================
//水电表读数查询
/// ========================

/// 水电表读数行（用于事务内查询）
#[derive(Debug)]
pub struct MeterReadingRow {
    pub id: i64,
    pub room_id: i64,
    pub year: i32,
    pub month: i32,
    pub water_reading: i64,
    pub electric_reading: i64,
    pub reading_date: String,
    pub operator: Option<String>,
    pub is_replacement: bool,
    pub is_deleted: bool,
}

impl MeterReadingRow {
    fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            year: row.get("year")?,
            month: row.get("month")?,
            water_reading: row.get("water_reading")?,
            electric_reading: row.get("electric_reading")?,
            reading_date: row.get("reading_date")?,
            operator: row.get("operator")?,
            is_replacement: row.get::<_, i32>("is_replacement")? != 0,
            is_deleted: row.get::<_, i32>("is_deleted")? != 0,
        })
    }
}

/// 事务版本：查询某房间某月是否已有抄表读数（幂等检查用）
pub fn get_meter_reading_by_room_month_tx(
    tx: &Transaction,
    room_id: i64,
    year: i32,
    month: i32,
) -> Result<Option<MeterReadingRow>> {
    let mut stmt = tx.prepare(
        "SELECT id, room_id, year, month, water_reading, electric_reading, \
                reading_date, operator, is_replacement, is_deleted \
         FROM meter_readings \
         WHERE room_id = ?1 AND year = ?2 AND month = ?3 AND is_deleted = 0 \
         LIMIT 1",
    )?;

    let result = stmt
        .query_row(params![room_id, year, month], MeterReadingRow::from_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(result)
}

/// 事务版本：获取房间最近一次抄表读数（从 meter_readings 表）
/// 用于抄表录入时的倒拨校验
pub fn get_latest_meter_reading_tx(
    tx: &Transaction,
    room_id: i64,
) -> Result<Option<MeterReadingRow>> {
    let mut stmt = tx.prepare(
        "SELECT id, room_id, year, month, water_reading, electric_reading, \
                reading_date, operator, is_replacement, is_deleted \
         FROM meter_readings \
         WHERE room_id = ?1 AND is_deleted = 0 \
         ORDER BY year DESC, month DESC, created_at DESC \
         LIMIT 1",
    )?;

    let result = stmt
        .query_row([room_id], MeterReadingRow::from_row)
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(result)
}

/// 事务版本：更新房间当前水电表读数（同步 meter_current）
pub fn update_room_meters_tx(
    tx: &Transaction,
    room_id: i64,
    water_meter: i64,
    electric_meter: i64,
) -> Result<()> {
    tx.execute(
        "UPDATE rooms SET water_meter_current = ?1, electric_meter_current = ?2, \
         updated_at = datetime('now') WHERE id = ?3",
        params![water_meter, electric_meter, room_id],
    )?;
    Ok(())
}

/// 乐观锁版本检查更新房间基础信息
/// 返回 Ok(affected) 成功时 affected > 0；版本冲突时返回 ConcurrentModification 错误
pub fn update_room_with_version_check_tx(
    tx: &Transaction,
    room_id: i64,
    expected_version: i64,
    base_rent: Option<i64>,
    property_fee: Option<i64>,
    water_meter_current: Option<i64>,
    electric_meter_current: Option<i64>,
    room_type: Option<&str>,
) -> Result<i64> {
    // 构建动态更新语句
    let mut updates = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(v) = base_rent {
        updates.push("base_rent = ?");
        params_vec.push(Box::new(v));
    }
    if let Some(v) = property_fee {
        updates.push("property_fee = ?");
        params_vec.push(Box::new(v));
    }
    if let Some(v) = water_meter_current {
        updates.push("water_meter_current = ?");
        params_vec.push(Box::new(v));
    }
    if let Some(v) = electric_meter_current {
        updates.push("electric_meter_current = ?");
        params_vec.push(Box::new(v));
    }
    if let Some(v) = room_type {
        updates.push("room_type = ?");
        params_vec.push(Box::new(v));
    }

    if updates.is_empty() {
        return Ok(0); // 没有要更新的字段
    }

    // 添加版本检查和版本递增
    updates.push("version = version + 1");
    updates.push("updated_at = datetime('now')");

    // 构建 WHERE 子句，包含版本检查
    let sql = format!(
        "UPDATE rooms SET {} WHERE id = ? AND version = ?",
        updates.join(", ")
    );

    // 构建参数列表
    params_vec.push(Box::new(room_id));
    params_vec.push(Box::new(expected_version));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let affected = tx.execute(&sql, params_refs.as_slice())
        .map_err(|e| crate::errors::AppError::Database(e))?;

    if affected == 0 {
        // 检查是因为房间不存在还是版本冲突
        let room_exists = tx
            .prepare("SELECT 1 FROM rooms WHERE id = ? AND is_deleted = 0")
            .and_then(|mut stmt| stmt.exists([room_id]))
            .unwrap_or(false);

        if room_exists {
            return Err(crate::errors::AppError::ConcurrentModification);
        }
        return Err(crate::errors::AppError::not_found("房间", room_id));
    }

    Ok(affected as i64)
}

/// ========================
//缴费查询
/// ========================

/// 查询缴费记录
pub fn list_payments(conn: &Connection, bill_id: Option<i64>) -> Result<Vec<Payment>> {
    let sql = match bill_id {
        Some(_) => "SELECT * FROM payments WHERE is_deleted = 0 AND bill_id = ? ORDER BY created_at DESC",
        None => "SELECT * FROM payments WHERE is_deleted = 0 ORDER BY created_at DESC",
    };

    let mut stmt = conn.prepare(sql)?;

    let payments = match bill_id {
        Some(id) => stmt.query_map([id], map_payment_row)?,
        None => stmt.query_map([], map_payment_row)?,
    }
    .collect::<SqliteResult<Vec<_>>>()
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(payments)
}

fn map_payment_row(row: &Row) -> SqliteResult<Payment> {
    Ok(Payment {
        id: row.get("id")?,
        bill_id: row.get("bill_id")?,
        room_id: row.get("room_id")?,
        lease_id: row.get("lease_id")?,
        amount: row.get("amount")?,
        payment_date: row.get("payment_date")?,
        payment_method: row.get("payment_method")?,
        wechat_amount: row.get("wechat_amount")?,
        alipay_amount: row.get("alipay_amount")?,
        cash_amount: row.get("cash_amount")?,
        bank_amount: row.get("bank_amount")?,
        deposit_deduct_amount: row.get("deposit_deduct_amount")?,
        payer_name: row.get("payer_name")?,
        confirmation_screenshot: row.get("confirmation_screenshot")?,
        operator: row.get("operator")?,
        notes: row.get("notes")?,
        is_deleted: row.get::<_, i32>("is_deleted")? != 0,
        created_at: row.get("created_at")?,
    })
}

/// ========================
//配置查询
/// ========================

/// 获取配置值
pub fn get_config_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT config_value FROM system_config WHERE config_key = ? AND is_active = 1",
    )?;

    let value = stmt
        .query_row([key], |row| row.get("config_value"))
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(value)
}

/// 事务版本：获取配置值（用于 TOCTOU 防护）
pub fn get_config_value_tx(tx: &Transaction, key: &str) -> Result<Option<String>> {
    let mut stmt = tx.prepare(
        "SELECT config_value FROM system_config WHERE config_key = ? AND is_active = 1",
    )?;

    let value = stmt
        .query_row([key], |row| row.get("config_value"))
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(value)
}

/// 获取所有配置
pub fn list_configs(conn: &Connection) -> Result<Vec<SystemConfig>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM system_config WHERE is_active = 1",
    )?;

    let configs = stmt
        .query_map([], |row| {
            Ok(SystemConfig {
                id: row.get("id")?,
                config_key: row.get("config_key")?,
                config_value: row.get("config_value")?,
                config_type: row.get("config_type")?,
                description: row.get("description")?,
                is_active: row.get::<_, i32>("is_active")? != 0,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(configs)
}

/// ========================
//房间状态日志查询
/// ========================

use crate::models::RoomStatusLog;

/// 查询房间状态变更日志
pub fn get_room_status_log(conn: &Connection, room_id: i64) -> Result<Vec<RoomStatusLog>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT * FROM room_status_log
        WHERE room_id = ?
        ORDER BY change_date DESC, created_at DESC
        "#,
    )?;

    let logs = stmt
        .query_map([room_id], |row| {
            Ok(RoomStatusLog {
                id: row.get("id")?,
                room_id: row.get("room_id")?,
                lease_id: row.get("lease_id")?,
                previous_status: get_room_status_nullable(row, "previous_status")?,
                new_status: row.get("new_status")?,
                trigger_type: row.get("trigger_type")?,
                tenant_id: row.get("tenant_id")?,
                tenant_name: row.get("tenant_name")?,
                change_date: row.get("change_date")?,
                effective_date: row.get("effective_date")?,
                operator: row.get("operator")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(logs)
}

/// ========================
//租客历史查询
/// ========================

use crate::models::TenantHistory;

/// 查询租客历史
pub fn get_tenant_history(conn: &Connection, tenant_id: i64) -> Result<Vec<TenantHistory>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT * FROM tenant_history
        WHERE tenant_id = ?
        ORDER BY event_date DESC, created_at DESC
        "#,
    )?;

    let history = stmt
        .query_map([tenant_id], |row| {
            Ok(TenantHistory {
                id: row.get("id")?,
                tenant_id: row.get("tenant_id")?,
                event_type: row.get("event_type")?,
                room_id: row.get("room_id")?,
                lease_id: row.get("lease_id")?,
                event_date: row.get("event_date")?,
                old_value: row.get("old_value")?,
                new_value: row.get("new_value")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(history)
}

/// ========================
//押金台账查询
/// ========================

use crate::models::DepositLedgerRow;

/// 查询押金台账
pub fn get_deposit_ledger(
    conn: &Connection,
    lease_id: Option<i64>,
    room_id: Option<i64>,
) -> Result<Vec<DepositLedgerRow>> {
    let sql = match (lease_id, room_id) {
        (Some(_), _) => {
            r#"
            SELECT dl.*, r.room_number, t.name as tenant_name
            FROM deposit_ledger dl
            JOIN rooms r ON r.id = dl.room_id
            LEFT JOIN leases l ON l.id = dl.lease_id
            LEFT JOIN tenants t ON t.id = l.tenant_id
            WHERE dl.lease_id = ?
            ORDER BY dl.created_at DESC
            "#
        }
        (None, Some(_)) => {
            r#"
            SELECT dl.*, r.room_number, t.name as tenant_name
            FROM deposit_ledger dl
            JOIN rooms r ON r.id = dl.room_id
            LEFT JOIN leases l ON l.id = dl.lease_id
            LEFT JOIN tenants t ON t.id = l.tenant_id
            WHERE dl.room_id = ?
            ORDER BY dl.created_at DESC
            "#
        }
        _ => {
            r#"
            SELECT dl.*, r.room_number, t.name as tenant_name
            FROM deposit_ledger dl
            JOIN rooms r ON r.id = dl.room_id
            LEFT JOIN leases l ON l.id = dl.lease_id
            LEFT JOIN tenants t ON t.id = l.tenant_id
            ORDER BY dl.created_at DESC
            LIMIT 100
            "#
        }
    };

    let mut stmt = conn.prepare(sql)?;

    let records = match (lease_id, room_id) {
        (Some(id), _) => stmt.query_map([id], map_deposit_row)?,
        (None, Some(id)) => stmt.query_map([id], map_deposit_row)?,
        _ => stmt.query_map([], map_deposit_row)?,
    }
    .collect::<SqliteResult<Vec<_>>>()
    .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(records)
}

fn map_deposit_row(row: &Row) -> SqliteResult<DepositLedgerRow> {
    Ok(DepositLedgerRow {
        id: row.get("id")?,
        lease_id: row.get("lease_id")?,
        room_id: row.get("room_id")?,
        room_number: row.get("room_number")?,
        tenant_name: row.get("tenant_name")?,
        transaction_type: row.get("transaction_type")?,
        amount: row.get("amount")?,
        balance: row.get("balance")?,
        reference_bill_id: row.get("reference_bill_id")?,
        reference_payment_id: row.get("reference_payment_id")?,
        operator: row.get("operator")?,
        transaction_date: row.get("transaction_date")?,
        notes: row.get("notes")?,
        created_at: row.get("created_at")?,
    })
}

// ========================
// 合同详细查询（带房间和租客信息）
// ========================

/// 查询合同详情（带房间和租客信息）
pub fn get_lease_detail(conn: &Connection, lease_id: i64) -> Result<Option<LeaseDetail>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            l.*,
            r.room_number,
            t.name as tenant_name,
            t.phone as tenant_phone,
            t.phone2 as tenant_phone2
        FROM leases l
        JOIN rooms r ON r.id = l.room_id
        JOIN tenants t ON t.id = l.tenant_id
        WHERE l.id = ? AND l.is_deleted = 0
        "#,
    )?;

    let detail = stmt
        .query_row([lease_id], |row| {
            Ok(LeaseDetail {
                id: row.get("id")?,
                room_id: row.get("room_id")?,
                tenant_id: row.get("tenant_id")?,
                room_number: row.get("room_number")?,
                tenant_name: row.get("tenant_name")?,
                tenant_phone: row.get("tenant_phone")?,
                tenant_phone2: row.get("tenant_phone2")?,
                contract_number: row.get("contract_number")?,
                start_date: row.get("start_date")?,
                end_date: row.get("end_date")?,
                monthly_rent: row.get("monthly_rent")?,
                property_fee: row.get("property_fee")?,
                deposit: row.get("deposit")?,
                deposit_received: row.get("deposit_received")?,
                deposit_balance: row.get("deposit_balance")?,
                deposit_status: row.get("deposit_status")?,
                move_in_date: row.get("move_in_date")?,
                move_out_date: row.get("move_out_date")?,
                termination_reason: row.get("termination_reason")?,
                status: row.get("status")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
            })
        })
        .optional()
        .map_err(|e| crate::errors::AppError::Database(e))?;

    Ok(detail)
}

// ========================
// 账单操作
// ========================

/// 确认账单全额支付（仅允许待缴费/部分支付状态）
pub fn confirm_bill_paid_tx(tx: &Transaction, bill_id: i64) -> Result<()> {
    let affected = tx.execute(
        "UPDATE monthly_bills SET status = ?1, actual_paid = total_amount, \
         updated_at = datetime('now') WHERE id = ?2 AND status IN (?3, ?4)",
        params![BillStatus::Paid.as_str(), bill_id, BillStatus::Pending.as_str(), BillStatus::Partial.as_str()],
    )?;
    if affected == 0 {
        return Err(crate::errors::AppError::InvalidStatus(
            "账单当前状态不允许确认支付".to_string()
        ));
    }
    Ok(())
}

/// 账单部分支付（仅允许待缴费/部分支付状态）
pub fn partial_pay_bill_tx(tx: &Transaction, bill_id: i64, amount: i64) -> Result<()> {
    let affected = tx.execute(
        "UPDATE monthly_bills SET actual_paid = actual_paid + ?1, \
         status = CASE \
            WHEN actual_paid + ?1 >= total_amount THEN ?2 \
            ELSE ?3 \
         END, \
         updated_at = datetime('now') WHERE id = ?4 AND status IN (?5, ?6)",
        params![amount, BillStatus::Paid.as_str(), BillStatus::Partial.as_str(), bill_id, BillStatus::Pending.as_str(), BillStatus::Partial.as_str()],
    )?;
    if affected == 0 {
        return Err(crate::errors::AppError::InvalidStatus(
            "账单当前状态不允许部分支付".to_string()
        ));
    }
    Ok(())
}

/// 作废账单（仅允许待缴费/部分支付状态）
pub fn void_bill_tx(tx: &Transaction, bill_id: i64) -> Result<()> {
    let affected = tx.execute(
        "UPDATE monthly_bills SET status = ?1, \
         updated_at = datetime('now') WHERE id = ?2 AND status IN (?3, ?4)",
        params![BillStatus::Voided.as_str(), bill_id, BillStatus::Pending.as_str(), BillStatus::Partial.as_str()],
    )?;
    if affected == 0 {
        return Err(crate::errors::AppError::InvalidStatus(
            "账单当前状态不允许作废".to_string()
        ));
    }
    Ok(())
}

/// 重新生成作废账单（将状态恢复为待缴费，清零实付金额）
pub fn regenerate_voided_bill_tx(tx: &Transaction, bill_id: i64) -> Result<()> {
    let affected = tx.execute(
        "UPDATE monthly_bills SET status = ?1, actual_paid = 0, \
         updated_at = datetime('now') WHERE id = ?2 AND status = ?3",
        params![BillStatus::Pending.as_str(), bill_id, BillStatus::Voided.as_str()],
    )?;
    if affected == 0 {
        return Err(crate::errors::AppError::InvalidStatus(
            "只有已作废的账单才能重新生成".to_string()
        ));
    }
    Ok(())
}
