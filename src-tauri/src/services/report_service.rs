//! 报表服务
//!
//! 月度汇总统计生成与查询

use crate::db::connection::HasConnection;
use crate::errors::Result;
use crate::models::report::{MonthlySummaryCache, MonthlySummaryListResponse, MonthlySummaryResponse};
use crate::models::RoomStatus;
use rusqlite::params;
use tracing::info;

/// 报表服务
pub struct ReportService;

impl ReportService {
    /// 生成月度汇总缓存
    ///
    /// 从 monthly_bills 表和 rooms 表计算实际统计数据
    /// ⚠️ TOCTOU 修复：读取和写入都在同一事务内
    pub fn generate_monthly_summary<C: HasConnection>(
        &self,
        ctx: &C,
        year_month: &str,
    ) -> Result<MonthlySummaryResponse> {
        info!("生成月度汇总: year_month={}", year_month);

        // 合并为单事务：读取 + 写入，防止 TOCTOU
        let summary = ctx.transaction(|tx| {
            let room_stats = self.get_room_stats_tx(tx)?;
            let fee_stats = self.get_fee_stats_tx(tx, year_month)?;

            let total_rented = room_stats.rented_count + room_stats.new_rented_count;
            let occupancy_rate = if room_stats.total_rooms > 0 {
                (total_rented as f64 / room_stats.total_rooms as f64) * 100.0
            } else {
                0.0
            };

            let cache = MonthlySummaryCache {
                id: 0,
                year_month: year_month.to_string(),
                total_rooms: room_stats.total_rooms,
                rented_count: room_stats.rented_count,
                new_rented_count: room_stats.new_rented_count,
                vacant_count: room_stats.vacant_count,
                violation_count: room_stats.violation_count,
                staff_count: room_stats.staff_count,
                management_count: room_stats.management_count,
                rent_total: fee_stats.rent_total,
                property_total: fee_stats.property_total,
                water_total: fee_stats.water_total,
                electric_total: fee_stats.electric_total,
                management_total: fee_stats.management_total,
                repair_total: fee_stats.repair_total,
                deposit_total: fee_stats.deposit_total,
                previous_balance_total: fee_stats.previous_balance_total,
                actual_paid_total: fee_stats.actual_paid_total,
                occupancy_rate,
                generated_at: None,
                updated_at: None,
            };

            tx.execute(
                r#"
                INSERT INTO monthly_summary_cache
                (year_month, total_rooms, rented_count, new_rented_count, vacant_count,
                 violation_count, staff_count, management_count,
                 rent_total, property_total, water_total, electric_total,
                 management_total, repair_total, deposit_total,
                 previous_balance_total, actual_paid_total, occupancy_rate,
                 generated_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                ON CONFLICT(year_month) DO UPDATE SET
                    total_rooms = excluded.total_rooms,
                    rented_count = excluded.rented_count,
                    new_rented_count = excluded.new_rented_count,
                    vacant_count = excluded.vacant_count,
                    violation_count = excluded.violation_count,
                    staff_count = excluded.staff_count,
                    management_count = excluded.management_count,
                    rent_total = excluded.rent_total,
                    property_total = excluded.property_total,
                    water_total = excluded.water_total,
                    electric_total = excluded.electric_total,
                    management_total = excluded.management_total,
                    repair_total = excluded.repair_total,
                    deposit_total = excluded.deposit_total,
                    previous_balance_total = excluded.previous_balance_total,
                    actual_paid_total = excluded.actual_paid_total,
                    occupancy_rate = excluded.occupancy_rate,
                    updated_at = datetime('now')
                "#,
                params![
                    cache.year_month, cache.total_rooms, cache.rented_count,
                    cache.new_rented_count, cache.vacant_count, cache.violation_count,
                    cache.staff_count, cache.management_count, cache.rent_total,
                    cache.property_total, cache.water_total, cache.electric_total,
                    cache.management_total, cache.repair_total, cache.deposit_total,
                    cache.previous_balance_total, cache.actual_paid_total, cache.occupancy_rate,
                ],
            )?;

            Ok::<_, crate::errors::AppError>(cache)
        })?;

        info!("月度汇总生成成功: year_month={}", year_month);
        Ok(MonthlySummaryResponse {
            success: true,
            data: Some(summary),
            message: None,
        })
    }

    /// 获取月度汇总
    pub fn get_summary_report<C: HasConnection>(
        &self,
        ctx: &C,
        year_month: &str,
    ) -> Result<MonthlySummaryResponse> {
        ctx.transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT * FROM monthly_summary_cache WHERE year_month = ?",
            )?;

            let result = stmt
                .query_row([year_month], |row| {
                    Ok(MonthlySummaryCache {
                        id: row.get("id")?,
                        year_month: row.get("year_month")?,
                        total_rooms: row.get("total_rooms")?,
                        rented_count: row.get("rented_count")?,
                        new_rented_count: row.get("new_rented_count")?,
                        vacant_count: row.get("vacant_count")?,
                        violation_count: row.get("violation_count")?,
                        staff_count: row.get("staff_count")?,
                        management_count: row.get("management_count")?,
                        rent_total: row.get("rent_total")?,
                        property_total: row.get("property_total")?,
                        water_total: row.get("water_total")?,
                        electric_total: row.get("electric_total")?,
                        management_total: row.get("management_total")?,
                        repair_total: row.get("repair_total")?,
                        deposit_total: row.get("deposit_total")?,
                        previous_balance_total: row.get("previous_balance_total")?,
                        actual_paid_total: row.get("actual_paid_total")?,
                        occupancy_rate: row.get("occupancy_rate")?,
                        generated_at: row.get("generated_at")?,
                        updated_at: row.get("updated_at")?,
                    })
                })
                .ok();

            match result {
                Some(data) => Ok(MonthlySummaryResponse {
                    success: true,
                    data: Some(data),
                    message: None,
                }),
                None => Ok(MonthlySummaryResponse {
                    success: false,
                    data: None,
                    message: Some(format!("未找到 {} 的汇总数据", year_month)),
                }),
            }
        })
    }

    /// 列出所有月度汇总
    pub fn list_summary_reports<C: HasConnection>(
        &self,
        ctx: &C,
    ) -> Result<MonthlySummaryListResponse> {
        ctx.transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT * FROM monthly_summary_cache ORDER BY year_month DESC",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(MonthlySummaryCache {
                    id: row.get("id")?,
                    year_month: row.get("year_month")?,
                    total_rooms: row.get("total_rooms")?,
                    rented_count: row.get("rented_count")?,
                    new_rented_count: row.get("new_rented_count")?,
                    vacant_count: row.get("vacant_count")?,
                    violation_count: row.get("violation_count")?,
                    staff_count: row.get("staff_count")?,
                    management_count: row.get("management_count")?,
                    rent_total: row.get("rent_total")?,
                    property_total: row.get("property_total")?,
                    water_total: row.get("water_total")?,
                    electric_total: row.get("electric_total")?,
                    management_total: row.get("management_total")?,
                    repair_total: row.get("repair_total")?,
                    deposit_total: row.get("deposit_total")?,
                    previous_balance_total: row.get("previous_balance_total")?,
                    actual_paid_total: row.get("actual_paid_total")?,
                    occupancy_rate: row.get("occupancy_rate")?,
                    generated_at: row.get("generated_at")?,
                    updated_at: row.get("updated_at")?,
                })
            })?;

            let mut summaries = Vec::new();
            for row in rows {
                summaries.push(row?);
            }

            Ok(MonthlySummaryListResponse {
                success: true,
                data: summaries,
                message: None,
            })
        })
    }

    /// 获取房间统计数据（实时计算）- 事务版本
    /// ⚠️ 修复：使用 RoomStatus 枚举值而非硬编码字符串
    fn get_room_stats_tx(&self, tx: &rusqlite::Transaction) -> Result<RoomStats> {
        let status_rented = RoomStatus::Rented.as_str();
        let status_new_rented = RoomStatus::NewRented.as_str();
        let status_vacant = RoomStatus::Vacant.as_str();
        let status_violation = RoomStatus::Violation.as_str();
        let status_staff = RoomStatus::Staff.as_str();
        let status_management = RoomStatus::Management.as_str();

        let sql = r#"
            SELECT
                COUNT(*) AS total_rooms,
                SUM(CASE WHEN status = ?1 THEN 1 ELSE 0 END) AS rented_count,
                SUM(CASE WHEN status = ?2 THEN 1 ELSE 0 END) AS new_rented_count,
                SUM(CASE WHEN status = ?3 THEN 1 ELSE 0 END) AS vacant_count,
                SUM(CASE WHEN status = ?4 THEN 1 ELSE 0 END) AS violation_count,
                SUM(CASE WHEN status = ?5 THEN 1 ELSE 0 END) AS staff_count,
                SUM(CASE WHEN status = ?6 THEN 1 ELSE 0 END) AS management_count
            FROM rooms
            WHERE is_deleted = 0
            "#;

        let mut stmt = tx.prepare(sql)?;

        let stats = stmt.query_row(
            params![status_rented, status_new_rented, status_vacant, status_violation, status_staff, status_management],
            |row| {
                Ok(RoomStats {
                    total_rooms: row.get("total_rooms")?,
                    rented_count: row.get("rented_count")?,
                    new_rented_count: row.get("new_rented_count")?,
                    vacant_count: row.get("vacant_count")?,
                    violation_count: row.get("violation_count")?,
                    staff_count: row.get("staff_count")?,
                    management_count: row.get("management_count")?,
                })
            }
        )?;

        Ok(stats)
    }

    /// 获取费用统计数据（从 monthly_bills 表计算）- 事务版本
    fn get_fee_stats_tx(&self, tx: &rusqlite::Transaction, year_month: &str) -> Result<FeeStats> {
        let mut stmt = tx.prepare(
            r#"
            SELECT
                COALESCE(SUM(rent_fee), 0) AS rent_total,
                COALESCE(SUM(property_fee), 0) AS property_total,
                COALESCE(SUM(water_fee), 0) AS water_total,
                COALESCE(SUM(electric_fee), 0) AS electric_total,
                COALESCE(SUM(management_fee), 0) AS management_total,
                COALESCE(SUM(repair_fee), 0) AS repair_total,
                COALESCE(SUM(deposit_fee), 0) AS deposit_total,
                COALESCE(SUM(previous_balance), 0) AS previous_balance_total,
                COALESCE(SUM(actual_paid), 0) AS actual_paid_total
            FROM monthly_bills
            WHERE year_month = ? AND is_deleted = 0
            "#,
        )?;

        let result = stmt.query_row([year_month], |row| {
            Ok(FeeStats {
                rent_total: row.get("rent_total")?,
                property_total: row.get("property_total")?,
                water_total: row.get("water_total")?,
                electric_total: row.get("electric_total")?,
                management_total: row.get("management_total")?,
                repair_total: row.get("repair_total")?,
                deposit_total: row.get("deposit_total")?,
                previous_balance_total: row.get("previous_balance_total")?,
                actual_paid_total: row.get("actual_paid_total")?,
            })
        });

        match result {
            Ok(stats) => Ok(stats),
            Err(_) => {
                // 如果该月没有账单，返回零值
                Ok(FeeStats {
                    rent_total: 0,
                    property_total: 0,
                    water_total: 0,
                    electric_total: 0,
                    management_total: 0,
                    repair_total: 0,
                    deposit_total: 0,
                    previous_balance_total: 0,
                    actual_paid_total: 0,
                })
            }
        }
    }
}

/// 房间统计
struct RoomStats {
    total_rooms: i64,
    rented_count: i64,
    new_rented_count: i64,
    vacant_count: i64,
    violation_count: i64,
    staff_count: i64,
    management_count: i64,
}

/// 费用统计
struct FeeStats {
    rent_total: i64,
    property_total: i64,
    water_total: i64,
    electric_total: i64,
    management_total: i64,
    repair_total: i64,
    deposit_total: i64,
    previous_balance_total: i64,
    actual_paid_total: i64,
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_occupancy_rate_calculation() {
        let total_rooms: i64 = 10;
        let rented_count: i64 = 6;
        let new_rented_count: i64 = 2;

        let total_rented = rented_count + new_rented_count;
        let occupancy_rate = (total_rented as f64 / total_rooms as f64) * 100.0;

        assert_eq!(occupancy_rate, 80.0);
    }

    #[test]
    fn test_zero_rooms_occupancy() {
        let total_rooms: i64 = 0;
        let rented_count: i64 = 0;
        let new_rented_count: i64 = 0;

        let total_rented = rented_count + new_rented_count;
        let occupancy_rate = if total_rooms > 0 {
            (total_rented as f64 / total_rooms as f64) * 100.0
        } else {
            0.0
        };

        assert_eq!(occupancy_rate, 0.0);
    }
}
