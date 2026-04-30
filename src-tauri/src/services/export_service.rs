//! 数据导出服务
//!
//! 负责从数据库导出各种类型的数据

use crate::db::HasConnection;
use crate::errors::{AppError, Result};
use crate::models::LeaseStatus;
use chrono::Local;
use rusqlite::Connection;
use rusqlite::params;
use serde::Serialize;
use serde_json::{json, Value};

/// 导出数据类型
#[derive(Debug, Clone, Serialize)]
pub struct ExportData {
    pub export_type: String,
    #[serde(rename = "exportTime")]
    pub export_time: String,
    #[serde(rename = "recordCount")]
    pub record_count: usize,
    pub data: Value,
}

impl ExportData {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"error":"序列化导出数据失败: {}"}}"#, e)
        })
    }
}

/// 导出服务
pub struct ExportService;

impl ExportService {
    /// 导出数据
    pub fn export_data<C: HasConnection>(
        &self,
        ctx: &C,
        data_type: &str,
        year_month: Option<&str>,
    ) -> Result<ExportData> {
        let conn = ctx.get_conn()?;
        match data_type {
            "rooms" => self.export_rooms(&conn),
            "tenants" => self.export_tenants(&conn),
            "bills" => self.export_bills(&conn, year_month),
            "payments" => self.export_payments(&conn, year_month),
            "summary" => self.export_summary(&conn),
            _ => Err(AppError::Business(format!("不支持的导出类型: {}", data_type))),
        }
    }

    /// 导出房间数据
    fn export_rooms(&self, conn: &Connection) -> Result<ExportData> {
        let mut stmt = conn.prepare(
            "SELECT r.id, r.room_number, r.floor, r.building, r.room_type,
                    r.base_rent, r.property_fee, r.deposit, r.status,
                    r.water_meter_current, r.electric_meter_current,
                    r.is_deleted, r.deleted_at, r.created_at, r.updated_at,
                    t.name as tenant_name, t.phone as tenant_phone,
                    l.contract_number, l.start_date, l.end_date
             FROM rooms r
             LEFT JOIN leases l ON r.id = l.room_id AND l.status = ?1
             LEFT JOIN tenants t ON l.tenant_id = t.id
             WHERE r.is_deleted = 0
             ORDER BY r.building, r.room_number",
        )?;

        let rows = stmt.query_map(params![LeaseStatus::Active.as_str()], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "room_number": row.get::<_, String>(1)?,
                "floor": row.get::<_, i32>(2)?,
                "building": row.get::<_, String>(3)?,
                "room_type": row.get::<_, String>(4)?,
                "base_rent": row.get::<_, i64>(5)?,
                "property_fee": row.get::<_, i64>(6)?,
                "deposit": row.get::<_, i64>(7)?,
                "status": row.get::<_, String>(8)?,
                "water_meter_current": row.get::<_, i64>(9)?,
                "electric_meter_current": row.get::<_, i64>(10)?,
                "is_deleted": row.get::<_, bool>(11)?,
                "deleted_at": row.get::<_, Option<String>>(12)?,
                "created_at": row.get::<_, Option<String>>(13)?,
                "updated_at": row.get::<_, Option<String>>(14)?,
                "tenant_name": row.get::<_, Option<String>>(15)?,
                "tenant_phone": row.get::<_, Option<String>>(16)?,
                "contract_number": row.get::<_, Option<String>>(17)?,
                "lease_start_date": row.get::<_, Option<String>>(18)?,
                "lease_end_date": row.get::<_, Option<String>>(19)?,
            }))
        })?;

        let mut data = Vec::new();
        for row in rows {
            data.push(row?);
        }

        Ok(ExportData {
            export_type: "rooms".to_string(),
            export_time: now_string(),
            record_count: data.len(),
            data: json!(data),
        })
    }

    /// 导出租客数据
    fn export_tenants(&self, conn: &Connection) -> Result<ExportData> {
        let mut stmt = conn.prepare(
            "SELECT id, name, phone, phone2, emergency_contact, emergency_phone,
                    is_deleted, deleted_at, created_at, updated_at
             FROM tenants
             WHERE is_deleted = 0
             ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "phone": row.get::<_, String>(2)?,
                "phone2": row.get::<_, Option<String>>(3)?,
                "emergency_contact": row.get::<_, Option<String>>(4)?,
                "emergency_phone": row.get::<_, Option<String>>(5)?,
                "is_deleted": row.get::<_, bool>(6)?,
                "deleted_at": row.get::<_, Option<String>>(7)?,
                "created_at": row.get::<_, Option<String>>(8)?,
                "updated_at": row.get::<_, Option<String>>(9)?,
            }))
        })?;

        let mut data = Vec::new();
        for row in rows {
            data.push(row?);
        }

        Ok(ExportData {
            export_type: "tenants".to_string(),
            export_time: now_string(),
            record_count: data.len(),
            data: json!(data),
        })
    }

    /// 导出账单数据
    fn export_bills(&self, conn: &Connection, year_month: Option<&str>) -> Result<ExportData> {
        let sql = "SELECT b.id, b.year_month, b.room_id, r.room_number, r.building,
                    t.name as tenant_name, t.phone as tenant_phone,
                    b.rent_fee, b.property_fee, b.water_fee, b.electric_fee,
                    b.management_fee, b.repair_fee, b.misc_fee, b.misc_fee_remark,
                    b.previous_balance, b.total_amount, b.actual_paid,
                    b.status, b.bill_type, b.due_date, b.paid_date,
                    b.is_deleted, b.is_archived, b.archived_at,
                    b.created_at, b.updated_at
             FROM monthly_bills b
             JOIN rooms r ON b.room_id = r.id
             LEFT JOIN leases l ON b.lease_id = l.id
             LEFT JOIN tenants t ON l.tenant_id = t.id
             WHERE b.is_deleted = 0
             ORDER BY b.year_month DESC, r.building, r.room_number";

        let mut stmt = conn.prepare(sql)?;

        let rows = stmt.query_map([], |row| bill_row_to_json(row))?;

        let mut data = Vec::new();
        for row in rows {
            data.push(row?);
        }

        if let Some(ym) = year_month {
            data.retain(|v| {
                v.get("year_month")
                    .and_then(|y| y.as_str())
                    .map(|y| y == ym)
                    .unwrap_or(false)
            });
        }

        Ok(ExportData {
            export_type: "bills".to_string(),
            export_time: now_string(),
            record_count: data.len(),
            data: serde_json::to_value(data).unwrap_or(json!([])),
        })
    }

    /// 导出缴费记录
    fn export_payments(&self, conn: &Connection, year_month: Option<&str>) -> Result<ExportData> {
        let sql = "SELECT p.id, p.bill_id, p.room_id, r.room_number, r.building,
                    p.amount, p.payment_date, p.payment_method,
                    b.year_month, b.total_amount,
                    p.is_deleted, p.created_at
             FROM payments p
             JOIN rooms r ON p.room_id = r.id
             LEFT JOIN monthly_bills b ON p.bill_id = b.id
             WHERE p.is_deleted = 0
             ORDER BY p.payment_date DESC";

        let mut stmt = conn.prepare(sql)?;

        let rows = stmt.query_map([], |row| payment_row_to_json(row))?;

        let mut data = Vec::new();
        for row in rows {
            data.push(row?);
        }

        if let Some(ym) = year_month {
            data.retain(|v| {
                v.get("year_month")
                    .and_then(|y| y.as_str())
                    .map(|y| y == ym)
                    .unwrap_or(false)
            });
        }

        Ok(ExportData {
            export_type: "payments".to_string(),
            export_time: now_string(),
            record_count: data.len(),
            data: serde_json::to_value(data).unwrap_or(json!([])),
        })
    }

    /// 导出汇总数据
    fn export_summary(&self, conn: &Connection) -> Result<ExportData> {
        let pending_str = crate::models::BillStatus::Pending.as_str();
        let partial_str = crate::models::BillStatus::Partial.as_str();
        let paid_str = crate::models::BillStatus::Paid.as_str();
        let mut stmt = conn.prepare(
            "SELECT year_month,
                    COUNT(*) as bill_count,
                    SUM(total_amount) as total_amount,
                    SUM(actual_paid) as total_paid,
                    SUM(CASE WHEN status IN (?1, ?2) THEN 1 ELSE 0 END) as pending_count,
                    SUM(CASE WHEN status = ?3 THEN 1 ELSE 0 END) as paid_count,
                    MIN(created_at) as first_created,
                    MAX(created_at) as last_created
             FROM monthly_bills
             WHERE is_deleted = 0
             GROUP BY year_month
             ORDER BY year_month DESC",
        )?;

        let rows = stmt.query_map(params![pending_str, partial_str, paid_str], |row| {
            Ok(json!({
                "year_month": row.get::<_, String>(0)?,
                "bill_count": row.get::<_, i64>(1)?,
                "total_amount": row.get::<_, Option<i64>>(2)?,
                "total_paid": row.get::<_, Option<i64>>(3)?,
                "pending_count": row.get::<_, i64>(4)?,
                "paid_count": row.get::<_, i64>(5)?,
                "first_created": row.get::<_, Option<String>>(6)?,
                "last_created": row.get::<_, Option<String>>(7)?,
            }))
        })?;

        let mut data = Vec::new();
        for row in rows {
            data.push(row?);
        }

        Ok(ExportData {
            export_type: "summary".to_string(),
            export_time: now_string(),
            record_count: data.len(),
            data: json!(data),
        })
    }
}

fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn bill_row_to_json(row: &rusqlite::Row) -> rusqlite::Result<serde_json::Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "year_month": row.get::<_, String>(1)?,
        "room_id": row.get::<_, i64>(2)?,
        "room_number": row.get::<_, String>(3)?,
        "building": row.get::<_, String>(4)?,
        "tenant_name": row.get::<_, Option<String>>(5)?,
        "tenant_phone": row.get::<_, Option<String>>(6)?,
        "rent_fee": row.get::<_, i64>(7)?,
        "property_fee": row.get::<_, i64>(8)?,
        "water_fee": row.get::<_, i64>(9)?,
        "electric_fee": row.get::<_, i64>(10)?,
        "management_fee": row.get::<_, i64>(11)?,
        "repair_fee": row.get::<_, i64>(12)?,
        "misc_fee": row.get::<_, i64>(13)?,
        "misc_fee_remark": row.get::<_, Option<String>>(14)?,
        "previous_balance": row.get::<_, i64>(15)?,
        "total_amount": row.get::<_, i64>(16)?,
        "actual_paid": row.get::<_, i64>(17)?,
        "status": row.get::<_, String>(18)?,
        "bill_type": row.get::<_, String>(19)?,
        "due_date": row.get::<_, Option<String>>(20)?,
        "paid_date": row.get::<_, Option<String>>(21)?,
        "is_deleted": row.get::<_, bool>(22)?,
        "is_archived": row.get::<_, bool>(23)?,
        "archived_at": row.get::<_, Option<String>>(24)?,
        "created_at": row.get::<_, Option<String>>(25)?,
        "updated_at": row.get::<_, Option<String>>(26)?,
    }))
}

fn payment_row_to_json(row: &rusqlite::Row) -> rusqlite::Result<serde_json::Value> {
    Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "bill_id": row.get::<_, Option<i64>>(1)?,
        "room_id": row.get::<_, i64>(2)?,
        "room_number": row.get::<_, String>(3)?,
        "building": row.get::<_, String>(4)?,
        "amount": row.get::<_, i64>(5)?,
        "payment_date": row.get::<_, String>(6)?,
        "payment_method": row.get::<_, String>(7)?,
        "year_month": row.get::<_, Option<String>>(8)?,
        "bill_total_amount": row.get::<_, Option<i64>>(9)?,
        "is_deleted": row.get::<_, bool>(10)?,
        "created_at": row.get::<_, Option<String>>(11)?,
    }))
}