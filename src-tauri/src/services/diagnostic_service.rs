//! 诊断与修复服务
//!
//! 负责数据库诊断和修复操作

use crate::db::create_connection;
use crate::errors::{AppError, Result};
use crate::models::{DepositStatus, LeaseStatus, RoomStatus};
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
    pub success: bool,
    pub fixed_count: i32,
    pub error_count: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTenantFix {
    pub room_number: String,
    pub tenant_name: String,
    pub phone: String,
}

impl From<(String, String, String)> for RoomTenantFix {
    fn from((room_number, tenant_name, phone): (String, String, String)) -> Self {
        RoomTenantFix {
            room_number,
            tenant_name,
            phone,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelDiagnostic {
    pub file_path: String,
    pub sheet_count: usize,
    pub row_count: usize,
    pub column_count: usize,
    pub room_numbers: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbDiagnostic {
    pub room_count: i64,
    pub tenant_count: i64,
    pub lease_count: i64,
    pub active_lease_count: i64,
    pub bill_count: i64,
    pub payment_count: i64,
    pub issues: Vec<String>,
}

/// 诊断服务
pub struct DiagnosticService;

impl DiagnosticService {
    /// 修复账单水电费
    ///
    /// 遍历所有账单，根据 meter_readings 重新计算水电费和管理费
    pub fn fix_meter_fees(&self) -> Result<DiagnosticReport> {
        let conn = create_connection()?;
        let tx = conn.unchecked_transaction()?;
        let result = self.fix_meter_fees_internal(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    fn fix_meter_fees_internal(&self, conn: &Connection) -> Result<DiagnosticReport> {
        let water_unit_price: i64 = conn
            .query_row(
                "SELECT CAST(config_value AS INTEGER) FROM system_config WHERE config_key = '水费单价'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(600);

        let electric_unit_price: i64 = conn
            .query_row(
                "SELECT CAST(config_value AS INTEGER) FROM system_config WHERE config_key = '电费单价'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(73);

        let management_unit_price: Option<i64> = conn
            .query_row(
                "SELECT CAST(config_value AS INTEGER) FROM system_config WHERE config_key = '管理费单价'",
                [],
                |row| row.get(0),
            )
            .ok();

        let bills: Vec<(i64, i64, String, i64, i64, i64, i64, i64, i64, Option<i64>)> = conn
            .prepare(
                "SELECT id, room_id, year_month, rent_fee, property_fee, water_fee, electric_fee, management_fee, total_amount, repair_fee FROM monthly_bills WHERE is_deleted = 0 ORDER BY year_month DESC",
            )?
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(AppError::Database)?;

        let mut fixed_count = 0;
        let mut error_count = 0;

        for (bill_id, room_id, ym, rent_fee, property_fee, _water_fee, _electric_fee, _management_fee, _total_amount, repair_fee) in bills {
            let (prev_water, prev_electric): (i64, i64) = conn
                .query_row(
                    "SELECT COALESCE(water_reading, 0), COALESCE(electric_reading, 0) FROM meter_readings WHERE room_id = ?1 AND year_month < ?2 AND is_deleted = 0 ORDER BY year_month DESC LIMIT 1",
                    params![room_id, ym],
                    |row| Ok((row.get(0).unwrap_or(0), row.get(1).unwrap_or(0))),
                )
                .unwrap_or((0, 0));

            let (curr_water, curr_electric): (i64, i64) = conn
                .query_row(
                    "SELECT COALESCE(water_reading, 0), COALESCE(electric_reading, 0) FROM meter_readings WHERE room_id = ?1 AND year_month = ?2 AND is_deleted = 0 LIMIT 1",
                    params![room_id, ym],
                    |row| Ok((row.get(0).unwrap_or(0), row.get(1).unwrap_or(0))),
                )
                .unwrap_or((0, 0));

            let water_usage = curr_water.saturating_sub(prev_water);
            let electric_usage = curr_electric.saturating_sub(prev_electric);

            let water_fee = water_usage
                .checked_mul(water_unit_price)
                .ok_or_else(|| AppError::business("水费计算溢出"))?;
            let electric_fee = electric_usage
                .checked_mul(electric_unit_price)
                .ok_or_else(|| AppError::business("电费计算溢出"))?;
            let mgmt_price = management_unit_price.unwrap_or(57);
            let management_fee = electric_usage
                .checked_mul(mgmt_price)
                .ok_or_else(|| AppError::business("管理费计算溢出"))?;

            let prev_balance: i64 = conn
                .query_row(
                    "SELECT COALESCE(previous_balance, 0) FROM monthly_bills WHERE id = ?1",
                    params![bill_id],
                    |row| row.get(0),
                )
                .unwrap_or(-1);

            let total_amount = prev_balance
                .checked_add(rent_fee)
                .and_then(|v| v.checked_add(property_fee))
                .and_then(|v| v.checked_add(water_fee))
                .and_then(|v| v.checked_add(electric_fee))
                .and_then(|v| v.checked_add(management_fee))
                .and_then(|v| v.checked_add(repair_fee.unwrap_or(0)))
                .ok_or_else(|| AppError::business("总金额计算溢出"))?;

            let after_total: i64 = conn
                .query_row(
                    "SELECT total_amount FROM monthly_bills WHERE id = ?1",
                    params![bill_id],
                    |row| row.get(0),
                )
                .unwrap_or(-1);

            if total_amount != after_total {
                match conn.execute(
                    "UPDATE monthly_bills SET water_fee = ?1, electric_fee = ?2, management_fee = ?3, total_amount = ?4, updated_at = datetime('now') WHERE id = ?5",
                    params![water_fee, electric_fee, management_fee, total_amount, bill_id],
                ) {
                    Ok(_) => {
                        info!(
                            "[fix_meter_fees] bill_id={} room={} {} water:{}x{}={} electric:{}x{}={} mgmt:{}x{}={} total={} rent={} prop={} DB_total={}",
                            bill_id, room_id, ym,
                            water_usage, water_unit_price, water_fee,
                            electric_usage, electric_unit_price, electric_fee,
                            electric_usage, mgmt_price, management_fee,
                            total_amount, rent_fee, property_fee, after_total
                        );
                        fixed_count += 1;
                    }
                    Err(e) => {
                        error_count += 1;
                        info!("[fix_meter_fees] bill_id={} 错误: {}", bill_id, e);
                    }
                }
            }
        }

        Ok(DiagnosticReport {
            success: true,
            fixed_count,
            error_count,
            message: format!("修复完成: {} 条成功, {} 条错误", fixed_count, error_count),
        })
    }

    /// 修复管理房间租客关联
    ///
    /// 为房间创建或更新租约关联
    pub fn fix_management_rooms(&self, fixes: Vec<RoomTenantFix>) -> Result<DiagnosticReport> {
        if fixes.is_empty() {
            return Ok(DiagnosticReport {
                success: true,
                fixed_count: 0,
                error_count: 0,
                message: "未提供修复列表".to_string(),
            });
        }

        let conn = create_connection()?;
        let tx = conn.unchecked_transaction()?;

        let mut fixed_count = 0;
        let mut error_count = 0;
        let mut messages = Vec::new();

        for fix in fixes {
            match self.fix_single_room(&tx, &fix) {
                Ok(msg) => {
                    fixed_count += 1;
                    messages.push(msg);
                }
                Err(e) => {
                    error_count += 1;
                    messages.push(format!("房间 {} 错误: {}", fix.room_number, e));
                }
            }
        }

        tx.execute("COMMIT", []).map_err(AppError::Database)?;

        info!("[FIX MANAGEMENT ROOMS]\n{}", messages.join("\n"));

        Ok(DiagnosticReport {
            success: true,
            fixed_count,
            error_count,
            message: messages.join("\n"),
        })
    }

    fn fix_single_room(&self, tx: &Transaction, fix: &RoomTenantFix) -> Result<String> {
        let room_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM rooms WHERE room_number = ?1 AND is_deleted = 0",
                params![fix.room_number],
                |row| row.get(0),
            )
            .ok();

        let room_id = match room_id {
            Some(id) => id,
            None => {
                return Ok(format!("房间 {} 不存在", fix.room_number));
            }
        };

        let tenant_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM tenants WHERE (name = ?1 OR phone = ?2) AND is_deleted = 0 LIMIT 1",
                params![fix.tenant_name, fix.phone],
                |row| row.get(0),
            )
            .ok();

        let tenant_id = match tenant_id {
            Some(id) => id,
            None => {
                return Ok(format!("房间 {} 租客 '{}' 不存在", fix.room_number, fix.tenant_name));
            }
        };

        let (existing_lease_id, existing_tenant_id): (Option<i64>, Option<i64>) = tx
            .query_row(
                "SELECT id, tenant_id FROM leases WHERE room_id = ?1 AND status = ?2 AND is_deleted = 0 LIMIT 1",
                params![room_id, LeaseStatus::Active.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
            .unwrap_or((None, None));

        if let (Some(lease_id), Some(current_tid)) = (existing_lease_id, existing_tenant_id) {
            if current_tid == tenant_id {
                Ok(format!(
                    "房间 {} 租约已正确关联租客 {} (tenant_id={})",
                    fix.room_number, fix.tenant_name, tenant_id
                ))
            } else {
                tx.execute(
                    "UPDATE leases SET tenant_id = ?1 WHERE id = ?2",
                    params![tenant_id, lease_id],
                )?;
                Ok(format!(
                    "房间 {} 租约tenant_id已修复 {} -> {}",
                    fix.room_number, current_tid, tenant_id
                ))
            }
        } else {
            match tx.execute(
                "INSERT INTO leases (room_id, tenant_id, start_date, end_date, monthly_rent, property_fee, deposit_received, deposit_balance, deposit_status, status, move_in_date, created_at) VALUES (?1, ?2, '2000-01-01', '2099-12-31', 0, 0, 0, 0, ?3, ?4, '2000-01-01', datetime('now'))",
                params![room_id, tenant_id, DepositStatus::Unreceived.as_str(), LeaseStatus::Active.as_str()],
            ) {
                Ok(_) => Ok(format!("房间 {} 已创建租约(tenant_id={})", fix.room_number, tenant_id)),
                Err(e) => Err(AppError::Database(e)),
            }
        }
    }

    pub fn diagnose_excel_file(file_path: &str) -> ExcelDiagnostic {
        let mut diag = ExcelDiagnostic {
            file_path: file_path.to_string(),
            sheet_count: 0,
            row_count: 0,
            column_count: 0,
            room_numbers: Vec::new(),
            errors: Vec::new(),
        };

        let mut workbook: calamine::Xlsx<_> = match calamine::open_workbook(std::path::Path::new(file_path)) {
            Ok(wb) => wb,
            Err(e) => {
                diag.errors.push(format!("读取工作表失败: {}", e));
                return diag;
            }
        };

        use calamine::Reader;
        let sheet_names = workbook.sheet_names().to_vec();
        diag.sheet_count = sheet_names.len();

        if let Some(name) = sheet_names.first().cloned() {
            if let Ok(range) = workbook.worksheet_range(&name) {
                let (end_row, end_col) = range.get_size();
                diag.row_count = end_row;
                diag.column_count = end_col;

                for row in range.rows().skip(1) {
                    if let Some(cell) = row.first() {
                        if let Some(val) = cell.to_string().trim().strip_prefix("'") {
                            if !val.is_empty() {
                                diag.room_numbers.push(val.to_string());
                            }
                        } else {
                            let s = cell.to_string().trim().to_string();
                            if !s.is_empty() {
                                diag.room_numbers.push(s);
                            }
                        }
                    }
                }
            }
        }

        diag
    }

    pub fn diagnose_database() -> Result<DbDiagnostic> {
        let conn = create_connection()?;
        let mut diag = DbDiagnostic {
            room_count: 0,
            tenant_count: 0,
            lease_count: 0,
            active_lease_count: 0,
            bill_count: 0,
            payment_count: 0,
            issues: Vec::new(),
        };

        diag.room_count = conn
            .query_row("SELECT COUNT(*) FROM rooms WHERE is_deleted = 0", [], |row| row.get(0))
            .unwrap_or(0);
        diag.tenant_count = conn
            .query_row("SELECT COUNT(*) FROM tenants WHERE is_deleted = 0", [], |row| row.get(0))
            .unwrap_or(0);
        diag.lease_count = conn
            .query_row("SELECT COUNT(*) FROM leases WHERE is_deleted = 0", [], |row| row.get(0))
            .unwrap_or(0);
        diag.active_lease_count = conn
            .query_row(
                "SELECT COUNT(*) FROM leases WHERE status = ? AND is_deleted = 0",
                [LeaseStatus::Active.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);
        diag.bill_count = conn
            .query_row("SELECT COUNT(*) FROM monthly_bills WHERE is_deleted = 0", [], |row| row.get(0))
            .unwrap_or(0);
        diag.payment_count = conn
            .query_row("SELECT COUNT(*) FROM payments WHERE is_deleted = 0", [], |row| row.get(0))
            .unwrap_or(0);

        let rented_str = RoomStatus::Rented.as_str();
        let new_rented_str = RoomStatus::NewRented.as_str();
        let violation_str = RoomStatus::Violation.as_str();
        let active_str = LeaseStatus::Active.as_str();
        let rooms_without_lease: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM rooms r WHERE r.is_deleted = 0 AND r.status IN ('{}','{}','{}') AND NOT EXISTS (SELECT 1 FROM leases l WHERE l.room_id = r.id AND l.status = '{}' AND l.is_deleted = 0)",
                    rented_str, new_rented_str, violation_str, active_str
                ),
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if rooms_without_lease > 0 {
            diag.issues.push(format!("有 {} 个在租/新租/违约房间缺少生效合同", rooms_without_lease));
        }

        Ok(diag)
    }

    pub fn diagnose_database_text() -> Result<String> {
        let diag = Self::diagnose_database()?;
        let mut out = String::new();
        out.push_str(&format!("房间: {}, 租客: {}, 合同: {} (生效: {})\n", diag.room_count, diag.tenant_count, diag.lease_count, diag.active_lease_count));
        out.push_str(&format!("账单: {}, 缴费: {}\n", diag.bill_count, diag.payment_count));
        if diag.issues.is_empty() {
            out.push_str("未发现问题\n");
        } else {
            for issue in &diag.issues {
                out.push_str(&format!("问题: {}\n", issue));
            }
        }
        Ok(out)
    }

    pub fn diagnose_room_detail(room_number: &str) -> Result<String> {
        let conn = create_connection()?;
        let mut out = String::new();

        let room: Option<(i64, String, String, String)> = conn
            .query_row(
                "SELECT id, room_number, status, room_type FROM rooms WHERE room_number = ?1 AND is_deleted = 0",
                params![room_number],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .ok();

        match room {
            Some((id, num, status, rtype)) => {
                out.push_str(&format!("房间: {} (ID={}, 状态={}, 类型={})\n", num, id, status, rtype));

                let leases: Vec<(i64, String, i64, String, String)> = conn
                    .prepare(
                        "SELECT id, status, tenant_id, start_date, end_date FROM leases WHERE room_id = ?1 AND is_deleted = 0 ORDER BY id DESC LIMIT 5"
                    )
                    .and_then(|mut stmt| {
                        let mut rows = stmt.query(params![id])?;
                        let mut result = Vec::new();
                        while let Some(r) = rows.next()? {
                            result.push((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?));
                        }
                        Ok(result)
                    })
                    .unwrap_or_default();

                out.push_str(&format!("合同: {} 条\n", leases.len()));
                for (lid, lstatus, tid, start, end) in &leases {
                    let tname: String = conn
                        .query_row("SELECT name FROM tenants WHERE id = ?1", params![tid], |row| row.get(0))
                        .unwrap_or_else(|_| "?".to_string());
                    out.push_str(&format!("  #{} 状态={} 租客={}({}) 期间={}-{}\n", lid, lstatus, tname, tid, start, end));
                }

                let bills: Vec<(i64, String, i64, i64)> = conn
                    .prepare(
                        "SELECT id, year_month, total_amount, actual_paid FROM monthly_bills WHERE room_id = ?1 AND is_deleted = 0 ORDER BY year_month DESC LIMIT 5",
                    )
                    .and_then(|mut stmt| {
                        let mut rows = stmt.query(params![id])?;
                        let mut result = Vec::new();
                        while let Some(r) = rows.next()? {
                            result.push((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?));
                        }
                        Ok(result)
                    })
                    .unwrap_or_default();

                out.push_str(&format!("账单: {} 条\n", bills.len()));
                for (bid, ym, total, paid) in &bills {
                    out.push_str(&format!("  #{} {} 总额={} 已付={}\n", bid, ym, total, paid));
                }
            }
            None => {
                out.push_str(&format!("房间 {} 不存在\n", room_number));
            }
        }

        Ok(out)
    }

    pub fn diagnose_meter_bill(room_id: i64) -> Result<String> {
        let conn = create_connection()?;
        let mut out = String::new();

        let room_number: String = conn
            .query_row("SELECT room_number FROM rooms WHERE id = ?1", params![room_id], |row| row.get(0))
            .unwrap_or_else(|_| "?".to_string());

        out.push_str(&format!("房间 {} (ID={}) 抄表账单诊断:\n", room_number, room_id));

        let readings: Vec<(String, i64, i64)> = conn
            .prepare(
                "SELECT year_month, water_reading, electric_reading FROM meter_readings WHERE room_id = ?1 AND is_deleted = 0 ORDER BY year_month DESC LIMIT 3",
            )
            .and_then(|mut stmt| {
                let mut rows = stmt.query(params![room_id])?;
                let mut result = Vec::new();
                while let Some(r) = rows.next()? {
                    result.push((r.get(0)?, r.get(1)?, r.get(2)?));
                }
                Ok(result)
            })
            .unwrap_or_default();

        out.push_str(&format!("抄表记录: {} 条\n", readings.len()));
        for (ym, w, e) in &readings {
            out.push_str(&format!("  {} 水={} 电={}\n", ym, w, e));
        }

        Ok(out)
    }
}