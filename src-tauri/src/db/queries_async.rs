//! 异步 SQL 查询模板（V3.0.0）
//!
//! 使用 sqlx 异步查询，Executor 泛型模式支持 &SqlitePool 和 &mut Transaction
//!
//! TOCTOU 防护规范同 legacy 版本：
//! 所有 Service 层的写前读取必须使用事务内查询，禁止在事务外读取后事务内写入。

use sqlx::{Executor, Row, Sqlite, SqlitePool};
use tracing::info;

use crate::errors::{AppError, Result};
use crate::models::{
    BillStatus, DepositLedgerRow, EscalationLevel, Lease, LeaseDetail, LeaseStatus, MonthlyBill,
    OverdueBill, Payment, Room, RoomResponse, RoomStatus, RoomStatusLog, SystemConfig, Tenant,
    TenantHistory,
};

fn parse_room_status(s: &str) -> std::result::Result<RoomStatus, AppError> {
    RoomStatus::from_str(s).map_err(|e| AppError::InvalidStatus(format!("房间状态解析失败: {}", e)))
}

fn parse_room_status_nullable(s: Option<String>) -> std::result::Result<Option<RoomStatus>, AppError> {
    match s {
        Some(s) if !s.is_empty() => Ok(Some(parse_room_status(&s)?)),
        _ => Ok(None),
    }
}

fn map_room_from_row(row: &sqlx::sqlite::SqliteRow) -> std::result::Result<Room, AppError> {
    Ok(Room {
        id: row.get("id"),
        room_number: row.get("room_number"),
        floor: row.get("floor"),
        building: row.get("building"),
        room_type: row.get("room_type"),
        base_rent: row.get("base_rent"),
        property_fee: row.get("property_fee"),
        deposit: row.get("deposit"),
        status: parse_room_status(&row.get::<String, _>("status"))?,
        water_meter_current: row.get("water_meter_current"),
        electric_meter_current: row.get("electric_meter_current"),
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
        version: row.try_get("version").unwrap_or(0),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_room_response_from_row(row: &sqlx::sqlite::SqliteRow) -> std::result::Result<RoomResponse, AppError> {
    Ok(RoomResponse {
        id: row.get("id"),
        room_number: row.get("room_number"),
        floor: row.get("floor"),
        building: row.get("building"),
        room_type: row.get("room_type"),
        base_rent_fen: row.get::<i64, _>("base_rent"),
        property_fee_fen: row.get::<i64, _>("property_fee"),
        deposit_fen: row.get::<i64, _>("deposit"),
        status: parse_room_status(&row.get::<String, _>("status"))?,
        water_meter_current: row.get("water_meter_current"),
        electric_meter_current: row.get("electric_meter_current"),
        tenant_name: row.get("tenant_name"),
        tenant_phone: row.get("tenant_phone"),
        lease_id: row.get("lease_id"),
        lease_start_date: row.get("lease_start_date"),
        lease_end_date: row.get("lease_end_date"),
        version: row.try_get("version").unwrap_or(0),
    })
}

fn map_tenant_from_row(row: &sqlx::sqlite::SqliteRow) -> Tenant {
    Tenant {
        id: row.get("id"),
        name: row.get("name"),
        phone: row.get("phone"),
        phone2: row.get("phone2"),
        emergency_contact: row.get("emergency_contact"),
        emergency_phone: row.get("emergency_phone"),
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_lease_from_row(row: &sqlx::sqlite::SqliteRow) -> Lease {
    Lease {
        id: row.get("id"),
        room_id: row.get("room_id"),
        tenant_id: row.get("tenant_id"),
        contract_number: row.get("contract_number"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        monthly_rent: row.get("monthly_rent"),
        property_fee: row.get("property_fee"),
        deposit: row.get("deposit"),
        deposit_received: row.get("deposit_received"),
        deposit_balance: row.get("deposit_balance"),
        deposit_status: row.get("deposit_status"),
        move_in_date: row.get("move_in_date"),
        move_out_date: row.get("move_out_date"),
        termination_reason: row.get("termination_reason"),
        status: row.get("status"),
        status_reason: row.get("status_reason"),
        notes: row.get("notes"),
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_bill_from_row(row: &sqlx::sqlite::SqliteRow) -> std::result::Result<MonthlyBill, AppError> {
    Ok(MonthlyBill {
        id: row.get("id"),
        year_month: row.get("year_month"),
        room_id: row.get("room_id"),
        lease_id: row.get("lease_id"),
        lease_start_date: row.get("lease_start_date"),
        lease_end_date: row.get("lease_end_date"),
        check_in_day: row.get("check_in_day"),
        check_out_day: row.get("check_out_day"),
        water_reading_prev: row.get("water_reading_prev"),
        water_reading_current: row.get("water_reading_current"),
        electric_reading_prev: row.get("electric_reading_prev"),
        electric_reading_current: row.get("electric_reading_current"),
        water_usage: row.get("water_usage"),
        electric_usage: row.get("electric_usage"),
        water_unit_price: row.get("water_unit_price"),
        electric_unit_price: row.get("electric_unit_price"),
        management_unit_price: row.get("management_unit_price"),
        rent_fee: row.get("rent_fee"),
        rent_days: row.get("rent_days"),
        rent_daily_rate: row.get("rent_daily_rate"),
        property_fee: row.get("property_fee"),
        water_fee: row.get("water_fee"),
        electric_fee: row.get("electric_fee"),
        management_fee: row.get("management_fee"),
        repair_fee: row.try_get("repair_fee").unwrap_or(0),
        misc_fee: row.try_get("misc_fee").unwrap_or(0),
        misc_fee_remark: row.try_get("misc_fee_remark").ok(),
        deposit_fee: row.get("deposit_fee"),
        previous_balance: row.get("previous_balance"),
        actual_paid: row.get("actual_paid"),
        total_amount: row.get("total_amount"),
        bill_type: row.get("bill_type"),
        room_status: parse_room_status(&row.get::<String, _>("room_status"))?,
        status: row.get("status"),
        due_date: row.get("due_date"),
        paid_date: row.get("paid_date"),
        bill_sequence: row.get("bill_sequence"),
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
        is_archived: row.try_get::<i32, _>("is_archived").unwrap_or(0) != 0,
        archived_at: row.try_get("archived_at").ok(),
        notes: row.get("notes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_payment_from_row(row: &sqlx::sqlite::SqliteRow) -> Payment {
    Payment {
        id: row.get("id"),
        bill_id: row.get("bill_id"),
        room_id: row.get("room_id"),
        lease_id: row.get("lease_id"),
        amount: row.get("amount"),
        payment_date: row.get("payment_date"),
        payment_method: row.get("payment_method"),
        wechat_amount: row.get("wechat_amount"),
        alipay_amount: row.get("alipay_amount"),
        cash_amount: row.get("cash_amount"),
        bank_amount: row.get("bank_amount"),
        deposit_deduct_amount: row.get("deposit_deduct_amount"),
        payer_name: row.get("payer_name"),
        confirmation_screenshot: row.get("confirmation_screenshot"),
        operator: row.get("operator"),
        notes: row.get("notes"),
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
        created_at: row.get("created_at"),
    }
}

// ========================
// 房间查询
// ========================

pub async fn list_rooms(pool: &SqlitePool) -> Result<Vec<RoomResponse>> {
    let rows = sqlx::query(
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
            WHERE rn = 1
        ) l ON l.room_id = r.id
        LEFT JOIN tenants t ON t.id = l.tenant_id AND t.is_deleted = 0
        WHERE r.is_deleted = 0
        ORDER BY COALESCE(r.floor, 0), r.room_number
        "#,
    )
    .bind(LeaseStatus::Active.as_str())
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_rooms 查询失败: {}", e)))?;

    let rooms: Vec<RoomResponse> = rows
        .iter()
        .map(map_room_response_from_row)
        .collect::<std::result::Result<Vec<_>, _>>()?;

    info!("[QUERY] list_rooms 返回 {} 条记录", rooms.len());
    Ok(rooms)
}

pub async fn get_room_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Room>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query("SELECT * FROM rooms WHERE id = ? AND is_deleted = 0")
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(format!("get_room_by_id 查询失败: {}", e)))?;

    row.map(|r| map_room_from_row(&r)).transpose()
}

pub async fn get_room_response_by_id(pool: &SqlitePool, id: i64) -> Result<Option<RoomResponse>> {
    let row = sqlx::query(
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
    )
    .bind(LeaseStatus::Active.as_str())
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("get_room_response_by_id 查询失败: {}", e)))?;

    row.map(|r| map_room_response_from_row(&r)).transpose()
}

pub async fn update_room_with_version_check<'e, E>(
    executor: E,
    room_id: i64,
    expected_version: i64,
    base_rent: Option<i64>,
    property_fee: Option<i64>,
    water_meter_current: Option<i64>,
    electric_meter_current: Option<i64>,
    room_type: Option<&str>,
) -> Result<i64>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let mut updates = Vec::new();
    let mut bind_values: Vec<Box<dyn Send + Sync>> = Vec::new();

    if let Some(v) = base_rent {
        updates.push("base_rent = ?");
        bind_values.push(Box::new(v) as Box<dyn Send + Sync>);
    }
    if let Some(v) = property_fee {
        updates.push("property_fee = ?");
        bind_values.push(Box::new(v) as Box<dyn Send + Sync>);
    }
    if let Some(v) = water_meter_current {
        updates.push("water_meter_current = ?");
        bind_values.push(Box::new(v) as Box<dyn Send + Sync>);
    }
    if let Some(v) = electric_meter_current {
        updates.push("electric_meter_current = ?");
        bind_values.push(Box::new(v) as Box<dyn Send + Sync>);
    }
    if let Some(v) = room_type {
        updates.push("room_type = ?");
        bind_values.push(Box::new(v.to_string()) as Box<dyn Send + Sync>);
    }

    if updates.is_empty() {
        return Ok(0);
    }

    updates.push("version = version + 1");
    updates.push("updated_at = datetime('now')");

    let sql = format!(
        "UPDATE rooms SET {} WHERE id = ? AND version = ?",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql);
    for _ in &bind_values {
        query = query.bind(0i64);
    }
    query = query.bind(room_id);
    query = query.bind(expected_version);

    let result = query
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(format!("update_room_with_version_check 失败: {}", e)))?;

    let affected = result.rows_affected() as i64;
    if affected == 0 {
        return Err(AppError::ConcurrentModification);
    }

    Ok(affected)
}

// ========================
// 租客查询
// ========================

pub async fn list_tenants(pool: &SqlitePool) -> Result<Vec<Tenant>> {
    let rows = sqlx::query(
        "SELECT * FROM tenants WHERE is_deleted = 0 ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_tenants 查询失败: {}", e)))?;

    Ok(rows.iter().map(map_tenant_from_row).collect())
}

pub async fn get_tenant_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Tenant>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query("SELECT * FROM tenants WHERE id = ? AND is_deleted = 0")
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(format!("get_tenant_by_id 查询失败: {}", e)))?;

    Ok(row.map(|r| map_tenant_from_row(&r)))
}

pub async fn create_tenant<'e, E>(executor: E, name: &str, phone: &str) -> Result<i64>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query("INSERT INTO tenants (name, phone) VALUES (?, ?)")
        .bind(name)
        .bind(phone)
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(format!("create_tenant 失败: {}", e)))?;

    Ok(result.last_insert_rowid())
}

pub async fn create_tenant_full<'e, E>(
    executor: E,
    name: &str,
    phone: &str,
    phone2: Option<&str>,
    emergency_contact: Option<&str>,
    emergency_phone: Option<&str>,
) -> Result<i64>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        r#"
        INSERT INTO tenants (name, phone, phone2, emergency_contact, emergency_phone)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(name)
    .bind(phone)
    .bind(phone2)
    .bind(emergency_contact)
    .bind(emergency_phone)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("create_tenant_full 失败: {}", e)))?;

    Ok(result.last_insert_rowid())
}

pub async fn update_tenant<'e, E>(
    executor: E,
    id: i64,
    name: Option<&str>,
    phone: Option<&str>,
    phone2: Option<&str>,
    emergency_contact: Option<&str>,
    emergency_phone: Option<&str>,
) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let mut updates = Vec::new();

    if name.is_some() {
        updates.push(("name", name.unwrap().to_string()));
    }
    if phone.is_some() {
        updates.push(("phone", phone.unwrap().to_string()));
    }
    if phone2.is_some() {
        updates.push(("phone2", phone2.unwrap_or("").to_string()));
    }
    if emergency_contact.is_some() {
        updates.push(("emergency_contact", emergency_contact.unwrap_or("").to_string()));
    }
    if emergency_phone.is_some() {
        updates.push(("emergency_phone", emergency_phone.unwrap_or("").to_string()));
    }

    if updates.is_empty() {
        return Ok(false);
    }

    let set_clauses: Vec<String> = updates.iter().enumerate().map(|(i, (col, _))| {
        format!("{} = ?{}", col, i + 1)
    }).collect();
    let sql = format!(
        "UPDATE tenants SET {}, updated_at = datetime('now') WHERE id = ?{}",
        set_clauses.join(", "),
        updates.len() + 1
    );

    let mut q = sqlx::query(&sql);
    for (_, val) in &updates {
        q = q.bind(val);
    }
    q = q.bind(id);

    let affected = q
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(format!("update_tenant 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

pub async fn delete_tenant<'e, E>(executor: E, id: i64) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let affected = sqlx::query(
        "UPDATE tenants SET is_deleted = 1, updated_at = datetime('now') WHERE id = ? AND is_deleted = 0",
    )
    .bind(id)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("delete_tenant 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

// ========================
// 合同查询
// ========================

pub async fn list_leases(pool: &SqlitePool) -> Result<Vec<Lease>> {
    let rows = sqlx::query(
        r#"
        SELECT * FROM leases
        WHERE is_deleted = 0
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_leases 查询失败: {}", e)))?;

    Ok(rows.iter().map(map_lease_from_row).collect())
}

pub async fn get_lease_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Lease>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query("SELECT * FROM leases WHERE id = ? AND is_deleted = 0")
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(format!("get_lease_by_id 查询失败: {}", e)))?;

    Ok(row.map(|r| map_lease_from_row(&r)))
}

pub async fn create_lease<'e, E>(
    executor: E,
    room_id: i64,
    tenant_id: i64,
    start_date: &str,
    monthly_rent: i64,
    property_fee: i64,
    deposit: i64,
    contract_number: Option<&str>,
    end_date: Option<&str>,
) -> Result<i64>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        r#"
        INSERT INTO leases
        (room_id, tenant_id, contract_number, start_date, end_date,
         monthly_rent, property_fee, deposit, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(room_id)
    .bind(tenant_id)
    .bind(contract_number)
    .bind(start_date)
    .bind(end_date)
    .bind(monthly_rent)
    .bind(property_fee)
    .bind(deposit)
    .bind(LeaseStatus::Draft.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("create_lease 失败: {}", e)))?;

    Ok(result.last_insert_rowid())
}

pub async fn update_lease_status<'e, E>(
    executor: E,
    id: i64,
    status: &str,
    expected_current_status: Option<&str>,
) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let affected = match expected_current_status {
        Some(expected) => {
            sqlx::query(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ? AND status = ?",
            )
            .bind(status)
            .bind(id)
            .bind(expected)
            .execute(executor)
            .await
        }
        None => {
            sqlx::query(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(status)
            .bind(id)
            .execute(executor)
            .await
        }
    }
    .map_err(|e| AppError::Database(format!("update_lease_status 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

pub async fn activate_lease<'e, E>(executor: E, id: i64) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let affected = sqlx::query(
        r#"
        UPDATE leases SET
            status = ?,
            updated_at = datetime('now')
        WHERE id = ? AND status = ?
        "#,
    )
    .bind(LeaseStatus::Active.as_str())
    .bind(id)
    .bind(LeaseStatus::Draft.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("activate_lease 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

pub async fn get_lease_detail(pool: &SqlitePool, lease_id: i64) -> Result<Option<LeaseDetail>> {
    let row = sqlx::query(
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
    )
    .bind(lease_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("get_lease_detail 查询失败: {}", e)))?;

    Ok(row.map(|r| LeaseDetail {
        id: r.get("id"),
        room_id: r.get("room_id"),
        tenant_id: r.get("tenant_id"),
        room_number: r.get("room_number"),
        tenant_name: r.get("tenant_name"),
        tenant_phone: r.get("tenant_phone"),
        tenant_phone2: r.get("tenant_phone2"),
        contract_number: r.get("contract_number"),
        start_date: r.get("start_date"),
        end_date: r.get("end_date"),
        monthly_rent: r.get("monthly_rent"),
        property_fee: r.get("property_fee"),
        deposit: r.get("deposit"),
        deposit_received: r.get("deposit_received"),
        deposit_balance: r.get("deposit_balance"),
        deposit_status: r.get("deposit_status"),
        move_in_date: r.get("move_in_date"),
        move_out_date: r.get("move_out_date"),
        termination_reason: r.get("termination_reason"),
        status: r.get("status"),
        notes: r.get("notes"),
        created_at: r.get("created_at"),
    }))
}

// ========================
// 账单查询
// ========================

pub async fn list_bills(pool: &SqlitePool, year_month: Option<String>) -> Result<Vec<MonthlyBill>> {
    let bills = match &year_month {
        Some(ym) => {
            let rows = sqlx::query(
                "SELECT * FROM monthly_bills WHERE is_deleted = 0 AND year_month = ? ORDER BY room_id",
            )
            .bind(ym)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("list_bills 查询失败: {}", e)))?;
            rows
        }
        None => {
            let rows = sqlx::query(
                "SELECT * FROM monthly_bills WHERE is_deleted = 0 ORDER BY year_month DESC, room_id",
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("list_bills 查询失败: {}", e)))?;
            rows
        }
    };

    bills
        .iter()
        .map(map_bill_from_row)
        .collect::<std::result::Result<Vec<_>, _>>()
}

pub async fn get_bill_by_room_month<'e, E>(
    executor: E,
    room_id: i64,
    year_month: &str,
) -> Result<Option<MonthlyBill>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT * FROM monthly_bills \
         WHERE room_id = ? AND year_month = ? AND is_deleted = 0 \
         ORDER BY bill_sequence DESC LIMIT 1",
    )
    .bind(room_id)
    .bind(year_month)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_bill_by_room_month 查询失败: {}", e)))?;

    row.map(|r| map_bill_from_row(&r)).transpose()
}

pub async fn get_latest_bill_for_room(pool: &SqlitePool, room_id: i64) -> Result<Option<MonthlyBill>> {
    let row = sqlx::query(
        "SELECT * FROM monthly_bills \
         WHERE room_id = ? AND is_deleted = 0 \
         ORDER BY year_month DESC, bill_sequence DESC LIMIT 1",
    )
    .bind(room_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("get_latest_bill_for_room 查询失败: {}", e)))?;

    row.map(|r| map_bill_from_row(&r)).transpose()
}

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
    pub room_number: String,
    pub building: String,
    pub tenant_name: Option<String>,
}

fn map_bill_row_from_row(row: &sqlx::sqlite::SqliteRow) -> BillRow {
    BillRow {
        id: row.get("id"),
        room_id: row.get("room_id"),
        year_month: row.get("year_month"),
        total_amount: row.get("total_amount"),
        actual_paid: row.get("actual_paid"),
        status: row.get("status"),
        due_date: row.get("due_date"),
        created_at: row.get("created_at"),
        room_number: row.get("room_number"),
        building: row.get("building"),
        tenant_name: row.get("tenant_name"),
    }
}

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

fn map_bill_detail_row_from_row(row: &sqlx::sqlite::SqliteRow) -> BillDetailRow {
    BillDetailRow {
        id: row.get("id"),
        room_id: row.get("room_id"),
        lease_id: row.get("lease_id"),
        year_month: row.get("year_month"),
        rent_fee: row.get("rent_fee"),
        property_fee: row.get("property_fee"),
        water_fee: row.get("water_fee"),
        electric_fee: row.get("electric_fee"),
        management_fee: row.get("management_fee"),
        repair_fee: row.try_get("repair_fee").unwrap_or(0),
        misc_fee: row.try_get("misc_fee").unwrap_or(0),
        misc_fee_remark: row.try_get("misc_fee_remark").ok(),
        previous_balance: row.get("previous_balance"),
        total_amount: row.get("total_amount"),
        actual_paid: row.get("actual_paid"),
        status: row.get("status"),
        due_date: row.get("due_date"),
        created_at: row.get("created_at"),
        water_reading_prev: row.get("water_reading_prev"),
        water_reading_current: row.get("water_reading_current"),
        electric_reading_prev: row.get("electric_reading_prev"),
        electric_reading_current: row.get("electric_reading_current"),
        room_number: row.get("room_number"),
        building: row.get("building"),
        tenant_name: row.get("tenant_name"),
        tenant_phone: row.get("tenant_phone"),
    }
}

#[derive(Debug)]
pub struct BillForAction {
    pub id: i64,
    pub room_id: i64,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub status: String,
}

pub async fn query_bills(
    pool: &SqlitePool,
    year: Option<i32>,
    month: Option<i32>,
    _room_id: Option<i64>,
    _status: Option<&str>,
    page: i32,
    page_size: i32,
) -> Result<(Vec<BillRow>, i32)> {
    let mut where_clauses = vec!["mb.is_deleted = 0".to_string()];

    if let Some(y) = year {
        if let Some(m) = month {
            where_clauses.push(format!("mb.year_month = '{:04}-{:02}'", y, m));
        } else {
            where_clauses.push(format!("mb.year_month LIKE '{:04}-%'", y));
        }
    }

    let where_sql = where_clauses.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) as cnt FROM monthly_bills mb WHERE {}", where_sql);
    let count: i32 = sqlx::query(&count_sql)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(format!("query_bills count 失败: {}", e)))?
        .get("cnt");

    let offset = (page - 1) * page_size;
    let data_sql = format!(
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

    let rows = sqlx::query(&data_sql)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("query_bills data 失败: {}", e)))?;

    let bills: Vec<BillRow> = rows.iter().map(map_bill_row_from_row).collect();
    Ok((bills, count))
}

pub async fn get_bill_by_id<'e, E>(executor: E, bill_id: i64) -> Result<Option<BillForAction>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT id, room_id, total_amount, actual_paid, status \
         FROM monthly_bills WHERE id = ? AND is_deleted = 0",
    )
    .bind(bill_id)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_bill_by_id 查询失败: {}", e)))?;

    Ok(row.map(|r| BillForAction {
        id: r.get("id"),
        room_id: r.get("room_id"),
        total_amount: r.get("total_amount"),
        actual_paid: r.get("actual_paid"),
        status: r.get("status"),
    }))
}

pub async fn get_bill_detail<'e, E>(executor: E, bill_id: i64) -> Result<Option<BillDetailRow>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        r#"
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
        WHERE mb.id = ? AND mb.is_deleted = 0
        LIMIT 1
        "#,
    )
    .bind(bill_id)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_bill_detail 查询失败: {}", e)))?;

    Ok(row.map(|r| map_bill_detail_row_from_row(&r)))
}

// ========================
// 账单归档
// ========================

pub async fn archive_bills_by_month<'e, E>(executor: E, year_month: &str) -> Result<i32>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        r#"
        UPDATE monthly_bills
        SET is_archived = 1, archived_at = datetime('now')
        WHERE year_month = ? AND is_deleted = 0 AND is_archived = 0
        "#,
    )
    .bind(year_month)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("archive_bills_by_month 失败: {}", e)))?;

    Ok(result.rows_affected() as i32)
}

pub async fn restore_bills_by_month<'e, E>(executor: E, year_month: &str) -> Result<i32>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        r#"
        UPDATE monthly_bills
        SET is_archived = 0, archived_at = NULL
        WHERE year_month = ? AND is_deleted = 0 AND is_archived = 1
        "#,
    )
    .bind(year_month)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("restore_bills_by_month 失败: {}", e)))?;

    Ok(result.rows_affected() as i32)
}

pub async fn get_archived_bills_count<'e, E>(executor: E, year_month: &str) -> Result<i32>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM monthly_bills WHERE year_month = ? AND is_archived = 1 AND is_deleted = 0",
    )
    .bind(year_month)
    .fetch_one(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_archived_bills_count 失败: {}", e)))?;

    Ok(row.get("cnt"))
}

pub async fn list_archived_year_months(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT year_month FROM monthly_bills WHERE is_archived = 1 AND is_deleted = 0 ORDER BY year_month DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_archived_year_months 查询失败: {}", e)))?;

    Ok(rows.iter().map(|r| r.get::<String, _>("year_month")).collect())
}

// ========================
// 账单操作
// ========================

pub async fn confirm_bill_paid<'e, E>(executor: E, bill_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        "UPDATE monthly_bills SET status = ?1, actual_paid = total_amount, \
         updated_at = datetime('now') WHERE id = ?2 AND status IN (?3, ?4)",
    )
    .bind(BillStatus::Paid.as_str())
    .bind(bill_id)
    .bind(BillStatus::Pending.as_str())
    .bind(BillStatus::Partial.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("confirm_bill_paid 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::InvalidStatus("账单当前状态不允许确认支付".to_string()));
    }
    Ok(())
}

pub async fn partial_pay_bill<'e, E>(executor: E, bill_id: i64, amount: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        "UPDATE monthly_bills SET actual_paid = actual_paid + ?1, \
         status = CASE \
            WHEN actual_paid + ?1 >= total_amount THEN ?2 \
            ELSE ?3 \
         END, \
         updated_at = datetime('now') WHERE id = ?4 AND status IN (?5, ?6)",
    )
    .bind(amount)
    .bind(BillStatus::Paid.as_str())
    .bind(BillStatus::Partial.as_str())
    .bind(bill_id)
    .bind(BillStatus::Pending.as_str())
    .bind(BillStatus::Partial.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("partial_pay_bill 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::InvalidStatus("账单当前状态不允许部分支付".to_string()));
    }
    Ok(())
}

pub async fn void_bill<'e, E>(executor: E, bill_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        "UPDATE monthly_bills SET status = ?1, \
         updated_at = datetime('now') WHERE id = ?2 AND status IN (?3, ?4)",
    )
    .bind(BillStatus::Voided.as_str())
    .bind(bill_id)
    .bind(BillStatus::Pending.as_str())
    .bind(BillStatus::Partial.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("void_bill 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::InvalidStatus("账单当前状态不允许作废".to_string()));
    }
    Ok(())
}

pub async fn regenerate_voided_bill<'e, E>(executor: E, bill_id: i64) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let result = sqlx::query(
        "UPDATE monthly_bills SET status = ?1, actual_paid = 0, \
         updated_at = datetime('now') WHERE id = ?2 AND status = ?3",
    )
    .bind(BillStatus::Pending.as_str())
    .bind(bill_id)
    .bind(BillStatus::Voided.as_str())
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("regenerate_voided_bill 失败: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::InvalidStatus("只有已作废的账单才能重新生成".to_string()));
    }
    Ok(())
}

// ========================
// 水电表读数
// ========================

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

fn map_meter_reading_from_row(row: &sqlx::sqlite::SqliteRow) -> MeterReadingRow {
    MeterReadingRow {
        id: row.get("id"),
        room_id: row.get("room_id"),
        year: row.get("year"),
        month: row.get("month"),
        water_reading: row.get("water_reading"),
        electric_reading: row.get("electric_reading"),
        reading_date: row.get("reading_date"),
        operator: row.get("operator"),
        is_replacement: row.get::<i32, _>("is_replacement") != 0,
        is_deleted: row.get::<i32, _>("is_deleted") != 0,
    }
}

pub async fn get_meter_reading_by_room_month<'e, E>(
    executor: E,
    room_id: i64,
    year: i32,
    month: i32,
) -> Result<Option<MeterReadingRow>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT id, room_id, year, month, water_reading, electric_reading, \
                reading_date, operator, is_replacement, is_deleted \
         FROM meter_readings \
         WHERE room_id = ? AND year = ? AND month = ? AND is_deleted = 0 \
         LIMIT 1",
    )
    .bind(room_id)
    .bind(year)
    .bind(month)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_meter_reading_by_room_month 查询失败: {}", e)))?;

    Ok(row.map(|r| map_meter_reading_from_row(&r)))
}

pub async fn get_latest_meter_reading<'e, E>(
    executor: E,
    room_id: i64,
) -> Result<Option<MeterReadingRow>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT id, room_id, year, month, water_reading, electric_reading, \
                reading_date, operator, is_replacement, is_deleted \
         FROM meter_readings \
         WHERE room_id = ? AND is_deleted = 0 \
         ORDER BY year DESC, month DESC, created_at DESC \
         LIMIT 1",
    )
    .bind(room_id)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_latest_meter_reading 查询失败: {}", e)))?;

    Ok(row.map(|r| map_meter_reading_from_row(&r)))
}

pub async fn update_room_meters<'e, E>(
    executor: E,
    room_id: i64,
    water_meter: i64,
    electric_meter: i64,
) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    sqlx::query(
        "UPDATE rooms SET water_meter_current = ?, electric_meter_current = ?, \
         updated_at = datetime('now') WHERE id = ?",
    )
    .bind(water_meter)
    .bind(electric_meter)
    .bind(room_id)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("update_room_meters 失败: {}", e)))?;

    Ok(())
}

// ========================
// 缴费查询
// ========================

pub async fn list_payments(pool: &SqlitePool, bill_id: Option<i64>) -> Result<Vec<Payment>> {
    let rows = match bill_id {
        Some(id) => {
            sqlx::query(
                "SELECT * FROM payments WHERE is_deleted = 0 AND bill_id = ? ORDER BY created_at DESC",
            )
            .bind(id)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT * FROM payments WHERE is_deleted = 0 ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| AppError::Database(format!("list_payments 查询失败: {}", e)))?;

    Ok(rows.iter().map(map_payment_from_row).collect())
}

// ========================
// 配置查询
// ========================

pub async fn get_config_value<'e, E>(executor: E, key: &str) -> Result<Option<String>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT config_value FROM system_config WHERE config_key = ? AND is_active = 1",
    )
    .bind(key)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_config_value 查询失败: {}", e)))?;

    Ok(row.and_then(|r| r.get::<Option<String>, _>("config_value")))
}

pub async fn list_configs(pool: &SqlitePool) -> Result<Vec<SystemConfig>> {
    let rows = sqlx::query(
        "SELECT * FROM system_config WHERE is_active = 1",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_configs 查询失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|r| SystemConfig {
            id: r.get("id"),
            config_key: r.get("config_key"),
            config_value: r.get("config_value"),
            config_type: r.get("config_type"),
            description: r.get("description"),
            is_active: r.get::<i32, _>("is_active") != 0,
        })
        .collect())
}

// ========================
// 房间状态日志
// ========================

pub async fn get_room_status_log(pool: &SqlitePool, room_id: i64) -> Result<Vec<RoomStatusLog>> {
    let rows = sqlx::query(
        r#"
        SELECT * FROM room_status_log
        WHERE room_id = ?
        ORDER BY change_date DESC, created_at DESC
        "#,
    )
    .bind(room_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("get_room_status_log 查询失败: {}", e)))?;

    rows.iter()
        .map(|r| {
            let prev: Option<String> = r.try_get("previous_status").ok().flatten();
            Ok(RoomStatusLog {
                id: r.get("id"),
                room_id: r.get("room_id"),
                lease_id: r.get("lease_id"),
                previous_status: parse_room_status_nullable(prev)?,
                new_status: parse_room_status(&r.get::<String, _>("new_status"))?,
                trigger_type: r.get("trigger_type"),
                tenant_id: r.get("tenant_id"),
                tenant_name: r.get("tenant_name"),
                change_date: r.get("change_date"),
                effective_date: r.get("effective_date"),
                operator: r.get("operator"),
                notes: r.get("notes"),
                created_at: r.get("created_at"),
            })
        })
        .collect()
}

// ========================
// 租客历史
// ========================

pub async fn get_tenant_history(pool: &SqlitePool, tenant_id: i64) -> Result<Vec<TenantHistory>> {
    let rows = sqlx::query(
        r#"
        SELECT * FROM tenant_history
        WHERE tenant_id = ?
        ORDER BY event_date DESC, created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("get_tenant_history 查询失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|r| TenantHistory {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            event_type: r.get("event_type"),
            room_id: r.get("room_id"),
            lease_id: r.get("lease_id"),
            event_date: r.get("event_date"),
            old_value: r.get("old_value"),
            new_value: r.get("new_value"),
            notes: r.get("notes"),
            created_at: r.get("created_at"),
        })
        .collect())
}

// ========================
// 押金台账
// ========================

pub async fn get_deposit_ledger(
    pool: &SqlitePool,
    lease_id: Option<i64>,
    room_id: Option<i64>,
) -> Result<Vec<DepositLedgerRow>> {
    let rows = match (lease_id, room_id) {
        (Some(_), _) => {
            sqlx::query(
                r#"
                SELECT dl.*, r.room_number, t.name as tenant_name
                FROM deposit_ledger dl
                JOIN rooms r ON r.id = dl.room_id
                LEFT JOIN leases l ON l.id = dl.lease_id
                LEFT JOIN tenants t ON t.id = l.tenant_id
                WHERE dl.lease_id = ?
                ORDER BY dl.created_at DESC
                "#,
            )
            .bind(lease_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("get_deposit_ledger 查询失败: {}", e)))?
        }
        (None, Some(_)) => {
            sqlx::query(
                r#"
                SELECT dl.*, r.room_number, t.name as tenant_name
                FROM deposit_ledger dl
                JOIN rooms r ON r.id = dl.room_id
                LEFT JOIN leases l ON l.id = dl.lease_id
                LEFT JOIN tenants t ON t.id = l.tenant_id
                WHERE dl.room_id = ?
                ORDER BY dl.created_at DESC
                "#,
            )
            .bind(room_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("get_deposit_ledger 查询失败: {}", e)))?
        }
        _ => {
            sqlx::query(
                r#"
                SELECT dl.*, r.room_number, t.name as tenant_name
                FROM deposit_ledger dl
                JOIN rooms r ON r.id = dl.room_id
                LEFT JOIN leases l ON l.id = dl.lease_id
                LEFT JOIN tenants t ON t.id = l.tenant_id
                ORDER BY dl.created_at DESC
                LIMIT 100
                "#,
            )
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("get_deposit_ledger 查询失败: {}", e)))?
        }
    };

    Ok(rows
        .iter()
        .map(|r| DepositLedgerRow {
            id: r.get("id"),
            lease_id: r.get("lease_id"),
            room_id: r.get("room_id"),
            room_number: r.get("room_number"),
            tenant_name: r.get("tenant_name"),
            transaction_type: r.get("transaction_type"),
            amount: r.get("amount"),
            balance: r.get("balance"),
            reference_bill_id: r.get("reference_bill_id"),
            reference_payment_id: r.get("reference_payment_id"),
            operator: r.get("operator"),
            transaction_date: r.get("transaction_date"),
            notes: r.get("notes"),
            created_at: r.get("created_at"),
        })
        .collect())
}

// ========================
// 租务 AI Agent
// ========================

pub async fn get_unpaid_bills<'e, E>(
    executor: E,
    year_month: Option<&str>,
) -> Result<Vec<OverdueBill>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let rows = sqlx::query(
        r#"
        SELECT
            b.id as bill_id,
            b.room_id,
            l.id as lease_id,
            r.room_number,
            r.building,
            t.name as tenant_name,
            t.phone as tenant_phone,
            b.year_month,
            b.total_amount,
            b.actual_paid,
            b.due_date,
            COALESCE(b.escalation_level, '') as escalation_level
        FROM monthly_bills b
        INNER JOIN rooms r ON r.id = b.room_id AND r.is_deleted = 0
        LEFT JOIN leases l ON l.id = b.lease_id AND l.is_deleted = 0 AND l.status = '生效中'
        LEFT JOIN tenants t ON t.id = l.tenant_id AND t.is_deleted = 0
        WHERE b.is_deleted = 0
          AND b.is_archived = 0
          AND b.status IN ('待缴费', '部分支付')
          AND (? IS NULL OR b.year_month = ?)
        ORDER BY b.due_date ASC NULLS LAST, r.room_number ASC
        "#,
    )
    .bind(year_month)
    .bind(year_month)
    .fetch_all(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_unpaid_bills 查询失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|r| {
            let total: i64 = r.get("total_amount");
            let paid: i64 = r.get("actual_paid");
            let level_str: String = r.get("escalation_level");
            OverdueBill {
                bill_id: r.get("bill_id"),
                room_id: r.get("room_id"),
                lease_id: r.get("lease_id"),
                room_number: r.get("room_number"),
                building: r.get("building"),
                tenant_name: r.get("tenant_name"),
                tenant_phone: r.get("tenant_phone"),
                year_month: r.get("year_month"),
                total_amount: total,
                actual_paid: paid,
                unpaid_amount: total.saturating_sub(paid),
                due_date: r.get("due_date"),
                overdue_days: 0,
                escalation_level: EscalationLevel::from_str(&level_str),
            }
        })
        .collect())
}

pub async fn update_bill_escalation<'e, E>(
    executor: E,
    bill_id: i64,
    level: &EscalationLevel,
) -> Result<()>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "UPDATE monthly_bills SET escalation_level = ?, last_reminder_at = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(level.as_str())
    .bind(now)
    .bind(bill_id)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("update_bill_escalation 失败: {}", e)))?;
    Ok(())
}

pub async fn get_bill_last_reminder_at<'e, E>(
    executor: E,
    bill_id: i64,
) -> Result<Option<String>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT last_reminder_at FROM monthly_bills WHERE id = ?",
    )
    .bind(bill_id)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_bill_last_reminder_at 查询失败: {}", e)))?;

    Ok(row.and_then(|r| r.get::<Option<String>, _>("last_reminder_at")))
}

pub async fn can_mark_violation<'e, E>(executor: E, room_id: i64) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT status FROM rooms WHERE id = ? AND is_deleted = 0",
    )
    .bind(room_id)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("can_mark_violation 查询失败: {}", e)))?;

    Ok(row
        .and_then(|r| r.get::<Option<String>, _>("status"))
        .map(|s| s == "在租" || s == "新租")
        .unwrap_or(false))
}

pub async fn mark_room_violation(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    room_id: i64,
    lease_id: Option<i64>,
) -> Result<()> {
    let current: String = sqlx::query(
        "SELECT status FROM rooms WHERE id = ? AND is_deleted = 0",
    )
    .bind(room_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(format!("mark_room_violation 查询失败: {}", e)))?
    .get("status");

    let room_status = parse_room_status(&current)
        .map_err(|s| AppError::InvalidStatus(format!("房间状态无效: {}", s)))?;

    if room_status != RoomStatus::Rented && room_status != RoomStatus::NewRented {
        return Err(AppError::InvalidStatus(format!(
            "房间状态为'{}'，不允许标记为违约",
            current
        )));
    }

    let new_status = RoomStatus::Violation.as_str();
    sqlx::query(
        "UPDATE rooms SET status = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(new_status)
    .bind(room_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(format!("mark_room_violation 更新房间失败: {}", e)))?;

    sqlx::query(
        "INSERT INTO room_status_log (room_id, lease_id, previous_status, new_status, trigger_type, change_date, created_at) \
         VALUES (?, ?, ?, ?, 'system', date('now'), datetime('now'))",
    )
    .bind(room_id)
    .bind(lease_id)
    .bind(&current)
    .bind(new_status)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(format!("mark_room_violation 插入日志失败: {}", e)))?;

    if let Some(lid) = lease_id {
        let current_lease: String = sqlx::query(
            "SELECT status FROM leases WHERE id = ?",
        )
        .bind(lid)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| AppError::Database(format!("mark_room_violation 查询合同失败: {}", e)))?
        .get("status");

        let lease_status = LeaseStatus::from_str(&current_lease)
            .map_err(|_| AppError::InvalidStatus(format!("合同状态无效: {}", current_lease)))?;
        if lease_status == LeaseStatus::Active {
            let new_lease_status = LeaseStatus::Violation.as_str();
            sqlx::query(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(new_lease_status)
            .bind(lid)
            .execute(&mut **tx)
            .await
            .map_err(|e| AppError::Database(format!("mark_room_violation 更新合同失败: {}", e)))?;
        }
    }

    Ok(())
}

pub async fn get_system_config_value<'e, E>(executor: E, key: &str) -> Result<Option<String>>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let row = sqlx::query(
        "SELECT config_value FROM system_config WHERE config_key = ? AND is_active = 1",
    )
    .bind(key)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("get_system_config_value 查询失败: {}", e)))?;

    Ok(row.and_then(|r| r.get::<Option<String>, _>("config_value")))
}

pub async fn create_collection_reminder<'e, E>(
    executor: E,
    bill_id: i64,
    room_id: i64,
    lease_id: Option<i64>,
    title: &str,
    message: &str,
    _escalation_level: &str,
) -> Result<i64>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let scheduled_date = chrono::Local::now().format("%Y-%m-%d").to_string();

    let result = sqlx::query(
        "INSERT INTO reminders (reminder_type, room_id, lease_id, title, message, scheduled_date, is_sent, is_read, created_at) \
         VALUES ('催租', ?, ?, ?, ?, ?, 0, 0, datetime('now'))",
    )
    .bind(room_id)
    .bind(lease_id)
    .bind(title)
    .bind(message)
    .bind(scheduled_date)
    .execute(executor)
    .await
    .map_err(|e| AppError::Database(format!("create_collection_reminder 失败: {}", e)))?;

    let id = result.last_insert_rowid();
    info!(
        "[CollectionAgent] 创建催租提醒 #{}: {} (bill={})",
        id, title, bill_id
    );
    Ok(id)
}
