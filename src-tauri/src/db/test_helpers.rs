//! 测试工具模块
//!
//! 提供测试数据库设置和数据准备工具

use rusqlite::{Connection, params};
use tempfile::NamedTempFile;

use super::migrations::run_migrations;

/// 创建测试数据库（临时文件）
pub fn create_test_db() -> Connection {
    let db_file = NamedTempFile::new().unwrap();
    let conn = Connection::open(db_file.path()).unwrap();
    run_migrations(&conn).unwrap();
    conn
}

/// 在内存中创建测试数据库
pub fn create_test_db_in_memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

/// 插入测试房间（默认 meter=100/200）
pub fn insert_test_room(
    conn: &Connection,
    room_number: &str,
    status: &str,
    base_rent: i64,
) -> i64 {
    insert_test_room_with_meters(conn, room_number, status, base_rent, 0, 100, 200)
}

/// 插入测试房间（带指定 meter读数和物业费）
pub fn insert_test_room_with_meters(
    conn: &Connection,
    room_number: &str,
    status: &str,
    base_rent: i64,
    property_fee: i64,
    water_meter: i64,
    electric_meter: i64,
) -> i64 {
    conn.execute(
        r#"
        INSERT INTO rooms (room_number, floor, building, base_rent, property_fee, deposit, status,
                         water_meter_current, electric_meter_current)
        VALUES (?, 1, 'A', ?, ?, 200000, ?, ?, ?)
        "#,
        params![room_number, base_rent, property_fee, status, water_meter, electric_meter],
    ).unwrap();
    conn.last_insert_rowid()
}

/// 插入测试租客
pub fn insert_test_tenant(conn: &Connection, name: &str, phone: &str) -> i64 {
    conn.execute(
        r#"
        INSERT INTO tenants (name, phone, phone2, emergency_contact, emergency_phone)
        VALUES (?, ?, '', '', '')
        "#,
        params![name, phone],
    ).unwrap();
    conn.last_insert_rowid()
}

/// 插入测试合同
pub fn insert_test_lease(
    conn: &Connection,
    room_id: i64,
    tenant_id: i64,
    status: &str,
    deposit: i64,
) -> i64 {
    conn.execute(
        r#"
        INSERT INTO leases (room_id, tenant_id, start_date, end_date, monthly_rent,
                          property_fee, deposit, deposit_balance, deposit_status, status,
                          contract_number, termination_reason)
        VALUES (?, ?, '2026-01-01', '2027-01-01', 300000, 5000, ?, 0, '未收取', ?, '测试合同', '')
        "#,
        params![room_id, tenant_id, deposit, status],
    ).unwrap();
    conn.last_insert_rowid()
}

/// 更新合同的入住日期（用于测试半月入住场景）
pub fn update_lease_move_in_date(conn: &Connection, lease_id: i64, move_in_date: &str) {
    conn.execute(
        "UPDATE leases SET move_in_date = ? WHERE id = ?",
        params![move_in_date, lease_id],
    ).unwrap();
}

/// 插入上期账单记录（用于设置上月抄表数据）
///
/// 同时插入 meter_readings 记录（供 generate_room_bill 新数据流使用）。
pub fn insert_test_meter_reading(
    conn: &Connection,
    room_id: i64,
    year_month: &str,
    water_reading: i64,
    electric_reading: i64,
) {
    conn.execute(
        r#"
        INSERT INTO monthly_bills
        (year_month, room_id, room_status,
         water_reading_prev, water_reading_current,
         electric_reading_prev, electric_reading_current,
         water_usage, electric_usage,
         water_unit_price, electric_unit_price, management_unit_price,
         rent_fee, rent_days, rent_daily_rate,
         property_fee, water_fee, electric_fee, management_fee,
         total_amount, previous_balance, actual_paid,
         status, bill_sequence)
        VALUES (?, ?, '在租', 0, ?, 0, ?, 0, 0, 600, 73, 57, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, '已支付', 1)
        "#,
        params![year_month, room_id, water_reading, electric_reading],
    ).unwrap();

    // 更新 rooms.meter_current（旧数据流兼容）
    conn.execute(
        "UPDATE rooms SET water_meter_current = ?, electric_meter_current = ? WHERE id = ?",
        params![water_reading, electric_reading, room_id],
    ).unwrap();

    // 插入 meter_readings（供 generate_room_bill 新数据流读取本期读数）
    // year_month 格式: "2026-03" → year=2026, month=3
    let parts: Vec<&str> = year_month.split('-').collect();
    let year: i32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let reading_date = format!("{}-15", year_month); // 假设每月15日抄表
    insert_test_meter_reading_for_month(conn, room_id, year, month, water_reading, electric_reading, &reading_date);
}

/// 插入抄表读数记录到 meter_readings 表（新的独立抄表数据流）
///
/// generate_room_bill 现在从 meter_readings 读取本期读数。
/// 此方法用于测试：新数据流下，每个账单月份需要先插入 meter_readings。
pub fn insert_test_meter_reading_for_month(
    conn: &Connection,
    room_id: i64,
    year: i32,
    month: i32,
    water_reading: i64,
    electric_reading: i64,
    reading_date: &str,
) {
    conn.execute(
        r#"INSERT INTO meter_readings
           (room_id, year, month, water_reading, electric_reading, reading_date, is_replacement, is_deleted)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)"#,
        params![room_id, year, month, water_reading, electric_reading, reading_date],
    ).unwrap();
}

/// 更新房间的当前抄表读数（不插入账单，仅更新 rooms 表）
pub fn update_room_meters(conn: &Connection, room_id: i64, water_meter: i64, electric_meter: i64) {
    conn.execute(
        "UPDATE rooms SET water_meter_current = ?, electric_meter_current = ? WHERE id = ?",
        params![water_meter, electric_meter, room_id],
    ).unwrap();
}
