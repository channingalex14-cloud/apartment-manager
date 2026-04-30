//! 导入服务
//!
//! 从 Excel 账单文件导入数据到数据库

use crate::db::create_connection;
use crate::errors::{AppError, Result};
use crate::models::{BillStatus, DepositStatus, LeaseStatus, RoomStatus};
use calamine::{open_workbook, Reader, Xlsx};
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use tracing::info;

/// 导入账单请求
#[derive(Debug, Clone, Deserialize)]
pub struct ImportBillRequest {
    pub file_path: String,
    pub year_month: String,
    pub operator: Option<String>,
}

/// 导入账单响应
#[derive(Debug, Clone, Serialize)]
pub struct ImportBillResponse {
    pub success: bool,
    pub imported_count: i32,
    pub skipped_count: i32,
    pub errors: Vec<String>,
    pub message: Option<String>,
}

/// Excel 账单行结构
#[derive(Debug, Clone)]
pub struct BillRow {
    pub room_number: String,
    pub room_type: String,
    pub tenant_name: String,
    pub phone: String,
    pub room_status: String,
    pub deposit: i64,
    pub rent: i64,
    pub property_fee: i64,
    pub lease_start: Option<String>,
    pub lease_end: Option<String>,
    pub water_reading_curr: i64,
    pub water_reading_prev: i64,
    pub water_usage: i64,
    pub water_fee: i64,
    pub electric_reading_curr: i64,
    pub electric_reading_prev: i64,
    pub electric_usage: i64,
    pub electric_fee: i64,
    pub management_fee: i64,
    pub repair_fee: i64,
    pub total_amount: i64,
    pub actual_paid: i64,
    pub payment_date: Option<String>,
    pub payment_method: String,
    pub payment_status: String,
    pub notes: String,
}

/// 导入服务
pub struct BillImportService;

impl BillImportService {
    /// 导入月度账单（主入口）
    pub fn import_monthly_bills(&self, req: &ImportBillRequest) -> Result<ImportBillResponse> {
        let file_path = req.file_path.clone();
        let year_month = req.year_month.clone();
        let operator = req.operator.clone().unwrap_or_else(|| "系统导入".to_string());

        let (year, month) = parse_year_month(&year_month)?;

        let rows = parse_excel_file(&file_path)?;
        info!("解析到 {} 条账单记录", rows.len());

        let mut conn = create_connection()?;
        let mut imported_count = 0;
        let mut skipped_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for row in rows {
            match self.import_single_bill(&mut conn, &row, &year_month, year, month, &operator) {
                Ok(_) => imported_count += 1,
                Err(e) => {
                    errors.push(e.user_message());
                    skipped_count += 1;
                }
            }
        }

        info!("导入完成: 成功 {} 条, 跳过 {} 条, 错误 {} 条", imported_count, skipped_count, errors.len());

        Ok(ImportBillResponse {
            success: errors.is_empty(),
            imported_count,
            skipped_count,
            errors,
            message: Some(format!("导入完成：成功 {} 条，跳过 {} 条", imported_count, skipped_count)),
        })
    }

    /// 导入单条账单
    fn import_single_bill(
        &self,
        conn: &mut Connection,
        row: &BillRow,
        year_month: &str,
        year: i32,
        month: u32,
        operator: &str,
    ) -> Result<()> {
        let tx = conn.transaction()?;

        let room_id = get_room_id_tx(&tx, &row.room_number)?
            .ok_or_else(|| AppError::Business(format!("房间 {} 不存在", row.room_number)))?;

        delete_existing_bill_for_room_month(&tx, room_id, year_month)?;

        let tenant_id = get_or_create_tenant(&tx, &row.tenant_name, &row.phone)?;

        let room_status_enum = normalize_status(&row.room_status);
        if matches!(room_status_enum, RoomStatus::NewRented | RoomStatus::Rented | RoomStatus::Management | RoomStatus::Staff) {
            ensure_lease_exists(&tx, room_id, tenant_id, row)?;
        }

        update_room_status(&tx, room_id, &row.room_status, &row.room_type, operator)?;

        upsert_meter_reading(&tx, room_id, year, month, row.water_reading_curr, row.electric_reading_curr)?;

        let bill_id = insert_bill(&tx, row, room_id, year_month)?;

        insert_payment(&tx, bill_id, room_id, row)?;

        tx.commit().map_err(AppError::Database)?;

        Ok(())
    }
}

fn parse_year_month(year_month: &str) -> Result<(i32, u32)> {
    let parts: Vec<&str> = year_month.split('-').collect();
    if parts.len() != 2 {
        return Err(AppError::Business(format!("无效的年月格式: {}，期望如 2026-03", year_month)));
    }
    let year: i32 = parts[0].parse().unwrap_or(2026);
    let month: u32 = parts[1].parse().unwrap_or(3);
    Ok((year, month))
}

fn yuan_to_fen(yuan: f64) -> i64 {
    let s = format!("{:.2}", yuan);
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts.first().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0) * 100;
    let dec_part = parts.get(1).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    if yuan < 0.0 { int_part - dec_part } else { int_part + dec_part }
}

pub fn parse_excel_file(path: &str) -> Result<Vec<BillRow>> {
    let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| {
        AppError::Business(format!("无法打开 Excel 文件: {} - {}", path, e))
    })?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Business("Excel 文件没有工作表".to_string()))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Business(format!("读取工作表失败: {}", e)))?;

    let mut rows: Vec<BillRow> = Vec::new();

    for (i, row) in range.rows().enumerate() {
        if i == 0 {
            continue;
        }
        if row.get(1).map(|c| c.to_string().trim().is_empty()).unwrap_or(true) {
            continue;
        }

        let get_str = |idx: usize| -> String {
            row.get(idx).map(|c| c.to_string().trim().to_string()).unwrap_or_default()
        };

        let get_f64 = |idx: usize| -> f64 {
            row.get(idx).and_then(|c| {
                match c {
                    calamine::Data::Float(n) => Some(*n),
                    calamine::Data::String(s) => s.trim().parse().ok(),
                    calamine::Data::Int(n) => Some(*n as f64),
                    _ => None,
                }
            }).unwrap_or(0.0)
        };

        let get_i64 = |idx: usize| -> i64 {
            row.get(idx).and_then(|c| {
                match c {
                    calamine::Data::Float(n) => Some(*n as i64),
                    calamine::Data::Int(n) => Some(*n),
                    calamine::Data::String(s) => s.trim().parse().ok(),
                    _ => None,
                }
            }).unwrap_or(0)
        };

        let room_number = get_str(1);
        if room_number.is_empty() || !room_number.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            continue;
        }

        let payment_status_raw = get_str(20);
        let payment_method_raw = get_str(19);

        rows.push(BillRow {
            room_number,
            room_type: get_str(2),
            tenant_name: get_str(3),
            phone: get_str(4),
            room_status: get_str(5),
            deposit: yuan_to_fen(get_f64(6)),
            rent: yuan_to_fen(get_f64(7)),
            property_fee: yuan_to_fen(get_f64(8)),
            lease_start: cell_to_date_string(row.get(9).unwrap_or(&calamine::Data::String(String::new()))),
            lease_end: cell_to_date_string(row.get(10).unwrap_or(&calamine::Data::String(String::new()))),
            water_reading_curr: get_i64(12),
            water_reading_prev: get_i64(13),
            water_usage: 0,
            water_fee: 0,
            electric_reading_curr: get_i64(14),
            electric_reading_prev: get_i64(15),
            electric_usage: 0,
            electric_fee: 0,
            management_fee: 0,
            repair_fee: yuan_to_fen(get_f64(16)),
            total_amount: 0,
            actual_paid: yuan_to_fen(get_f64(17)),
            payment_date: cell_to_date_string(row.get(18).unwrap_or(&calamine::Data::String(String::new()))),
            payment_method: if payment_method_raw.is_empty() { "未知".to_string() } else { payment_method_raw },
            payment_status: if payment_status_raw.is_empty() { BillStatus::Pending.as_str().to_string() } else { payment_status_raw },
            notes: get_str(21),
        });
    }

    Ok(rows)
}

fn excel_date_to_string(n: f64) -> Option<String> {
    if n < 2.0 {
        return None;
    }
    let days = n as i64;
    let date = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?
        .checked_add_signed(chrono::Duration::days(days))?;
    Some(date.format("%Y-%m-%d").to_string())
}

fn cell_to_date_string(cell: &calamine::Data) -> Option<String> {
    match cell {
        calamine::Data::Float(n) => excel_date_to_string(*n),
        calamine::Data::String(s) => {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Some(d.format("%Y-%m-%d").to_string())
            } else if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
                Some(d.format("%Y-%m-%d").to_string())
            } else {
                None
            }
        }
        calamine::Data::DateTime(n) => {
            n.as_datetime().map(|dt| dt.format("%Y-%m-%d").to_string())
        }
        _ => None,
    }
}

fn get_or_create_tenant(tx: &Transaction, name: &str, phone: &str) -> Result<i64> {
    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM tenants WHERE phone = ?1 AND is_deleted = 0 LIMIT 1",
            params![phone],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        return Ok(id);
    }

    tx.execute("INSERT INTO tenants (name, phone) VALUES (?1, ?2)", params![name, phone])
        .map_err(AppError::Database)?;

    Ok(tx.last_insert_rowid())
}

fn get_room_id_tx(tx: &Transaction, room_number: &str) -> Result<Option<i64>> {
    let id: Option<i64> = tx
        .query_row(
            "SELECT id FROM rooms WHERE room_number = ?1 AND is_deleted = 0 LIMIT 1",
            params![room_number],
            |row| row.get(0),
        )
        .ok();
    Ok(id)
}

fn insert_bill(
    tx: &Transaction,
    row: &BillRow,
    room_id: i64,
    year_month: &str,
) -> Result<i64> {
    let water_unit_price = 600i64;
    let electric_unit_price = 73i64;

    tx.execute(
        r#"
        INSERT INTO monthly_bills (
            year_month, room_id,
            lease_start_date, lease_end_date,
            water_reading_prev, water_reading_current,
            electric_reading_prev, electric_reading_current,
            water_usage, electric_usage,
            water_unit_price, electric_unit_price,
            rent_fee, property_fee, repair_fee,
            total_amount, actual_paid,
            room_status, status, notes,
            created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
            ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, datetime('now')
        )
        "#,
        params![
            year_month, room_id,
            row.lease_start, row.lease_end,
            row.water_reading_prev, row.water_reading_curr,
            row.electric_reading_prev, row.electric_reading_curr,
            row.water_usage, row.electric_usage,
            water_unit_price, electric_unit_price,
            row.rent, row.property_fee, row.repair_fee,
            row.actual_paid, row.actual_paid,
            row.room_status,
            if row.payment_status == BillStatus::Paid.as_str() { BillStatus::Paid.as_str() } else { BillStatus::Pending.as_str() },
            row.notes,
        ],
    )
    .map_err(AppError::Database)?;

    Ok(tx.last_insert_rowid())
}

fn insert_payment(tx: &Transaction, bill_id: i64, room_id: i64, row: &BillRow) -> Result<()> {
    if row.payment_status != BillStatus::Paid.as_str() || row.actual_paid <= 0 {
        return Ok(());
    }

    let amount = row.actual_paid;

    let payment_method = match row.payment_method.as_str() {
        "微信" | "支付宝" | "银行卡" | "现金" | "商家码" | "押金抵扣" => row.payment_method.as_str(),
        _ => "微信",
    };

    tx.execute(
        r#"
        INSERT INTO payments (bill_id, room_id, amount, payment_date, payment_method, payer_name, operator, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
        "#,
        params![bill_id, room_id, amount, row.payment_date, payment_method, row.tenant_name, row.notes],
    )
    .map_err(AppError::Database)?;

    Ok(())
}

fn update_room_status(tx: &Transaction, room_id: i64, status: &str, room_type: &str, operator: &str) -> Result<()> {
    let old_status_str: String = tx
        .query_row("SELECT status FROM rooms WHERE id = ?", [room_id], |row| row.get("status"))
        .map_err(AppError::Database)?;

    let new_status = normalize_status(status);

    // 校验房间状态转换合法性
    let old_status = RoomStatus::from_str(&old_status_str)
        .map_err(|e| AppError::invalid_status(e))?;
    if !old_status.can_transition_to(&new_status) {
        return Err(AppError::invalid_status(&format!(
            "房间状态不允许从 '{}' 转换为 '{}'", old_status_str, new_status.as_str()
        )));
    }

    tx.execute(
        "UPDATE rooms SET status = ?1, room_type = ?2, updated_at = datetime('now') WHERE id = ?3",
        params![new_status, room_type, room_id],
    )
    .map_err(AppError::Database)?;

    tx.execute(
        r#"
        INSERT INTO room_status_log (room_id, previous_status, new_status, trigger_type, change_date, operator)
        VALUES (?1, ?2, ?3, '批量导入', datetime('now'), ?4)
        "#,
        params![room_id, old_status_str, new_status.as_str(), operator],
    )
    .map_err(AppError::Database)?;

    Ok(())
}

pub fn normalize_status(status: &str) -> RoomStatus {
    match status {
        "在租" => RoomStatus::Rented,
        "新租" => RoomStatus::NewRented,
        "空房" | "空置" | "空闲" => RoomStatus::Vacant,
        "退房" | "已退" | "退租" => RoomStatus::PendingClean,
        "员工" => RoomStatus::Staff,
        "管理" => RoomStatus::Management,
        "违约" => RoomStatus::Violation,
        "维修中" | "维修" => RoomStatus::Maintenance,
        "待清洁" | "清洁" => RoomStatus::PendingClean,
        _ => {
            if !status.is_empty() {
                info!("未知房态 '{}' 映射为 '空房'", status);
            }
            RoomStatus::Vacant
        }
    }
}

fn delete_existing_bill_for_room_month(tx: &Transaction, room_id: i64, year_month: &str) -> Result<()> {
    let mut stmt = tx.prepare("SELECT id FROM monthly_bills WHERE room_id = ?1 AND year_month = ?2")?;
    let bill_ids: Vec<i64> = stmt
        .query_map(params![room_id, year_month], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    for bill_id in &bill_ids {
        tx.execute(
            "UPDATE payments SET is_deleted = 1, deleted_at = datetime('now') WHERE bill_id = ?1",
            params![bill_id],
        )?;
    }

    tx.execute(
        "UPDATE monthly_bills SET is_deleted = 1, deleted_at = datetime('now'), deleted_by = 'import' WHERE room_id = ?1 AND year_month = ?2",
        params![room_id, year_month],
    )?;

    Ok(())
}

fn upsert_meter_reading(tx: &Transaction, room_id: i64, year: i32, month: u32, water_reading: i64, electric_reading: i64) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO meter_readings (room_id, year, month, water_reading, electric_reading, reading_date, operator)
        VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), '系统导入')
        ON CONFLICT(room_id, year, month) DO UPDATE SET
            water_reading = excluded.water_reading,
            electric_reading = excluded.electric_reading,
            updated_at = datetime('now')
        "#,
        params![room_id, year, month, water_reading, electric_reading],
    )
    .map_err(AppError::Database)?;
    Ok(())
}

fn ensure_lease_exists(tx: &Transaction, room_id: i64, tenant_id: i64, row: &BillRow) -> Result<()> {
    let existing: i32 = tx
        .query_row(
            "SELECT COUNT(*) FROM leases WHERE room_id = ? AND status = ? AND is_deleted = 0",
            params![room_id, LeaseStatus::Active.as_str()],
            |_row| _row.get(0),
        )
        .unwrap_or(1);

    if existing == 0 {
        let deposit_received = row.deposit;
        let monthly_rent = row.rent;
        let property_fee = row.property_fee;
        let lease_start = row.lease_start.clone().unwrap_or_else(|| "2000-01-01".to_string());
        let lease_end = row.lease_end.clone().unwrap_or_else(|| "2099-12-31".to_string());

        tx.execute(
            r#"
            INSERT INTO leases (
                room_id, tenant_id, start_date, end_date,
                monthly_rent, property_fee, deposit_received, deposit_balance, deposit_status,
                status, move_in_date, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9, ?3, datetime('now'))
            "#,
            params![room_id, tenant_id, lease_start, lease_end, monthly_rent, property_fee, deposit_received, DepositStatus::Received.as_str(), LeaseStatus::Active.as_str()],
        )
        .map_err(AppError::Database)?;
    }

    Ok(())
}