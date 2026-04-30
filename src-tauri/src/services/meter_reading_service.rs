//! 抄表服务
//!
//! 水电表读数录入、批量录入、倒拨校验

use crate::db::queries;
use crate::db::connection::HasConnection;
use crate::errors::{AppError, Result};
use crate::models::meter_reading::{MeterReadingRequest, MeterReadingResponse, BatchMeterReadingRequest};

/// 抄表服务
pub struct MeterReadingService;

impl MeterReadingService {
    /// 单条录入抄表读数
    pub fn record_meter_reading<C: HasConnection>(
        &self,
        ctx: &C,
        req: &MeterReadingRequest,
    ) -> Result<MeterReadingResponse> {
        ctx.transaction(|tx| {
            // 1. 验证房间存在
            let _room = queries::get_room_by_id_tx(tx, req.room_id)?
                .ok_or_else(|| AppError::not_found("房间", req.room_id))?;

            // 2. 幂等检查：同房间同年月是否已有录入
            let existing = queries::get_meter_reading_by_room_month_tx(
                tx, req.room_id, req.year, req.month,
            )?;
            if existing.is_some() {
                return Ok(MeterReadingResponse {
                    success: false,
                    message: format!(
                        "房间 {} 的 {}-{:02} 读数已录入",
                        req.room_id, req.year, req.month
                    ),
                    id: None,
                });
            }

            // 3. 倒拨校验（仅在非换表场景）
            if !req.is_replacement {
                let latest = queries::get_latest_meter_reading_tx(tx, req.room_id)?;
                if let Some(prev) = latest {
                    if req.water_reading < prev.water_reading {
                        return Ok(MeterReadingResponse {
                            success: false,
                            message: format!(
                                "水表读数不能小于上期: {} < {}",
                                req.water_reading, prev.water_reading
                            ),
                            id: None,
                        });
                    }
                    if req.electric_reading < prev.electric_reading {
                        return Ok(MeterReadingResponse {
                            success: false,
                            message: format!(
                                "电表读数不能小于上期: {} < {}",
                                req.electric_reading, prev.electric_reading
                            ),
                            id: None,
                        });
                    }
                }
            }

            // 4. 插入抄表记录
            let is_replacement: i32 = if req.is_replacement { 1 } else { 0 };
            tx.execute(
                r#"INSERT INTO meter_readings
                   (room_id, year, month, water_reading, electric_reading,
                    reading_date, operator, is_replacement, is_deleted,
                    created_at, updated_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, datetime('now'), datetime('now'))"#,
                rusqlite::params![
                    req.room_id, req.year, req.month,
                    req.water_reading, req.electric_reading,
                    req.reading_date, req.operator,
                    is_replacement,
                ],
            )?;
            let id = tx.last_insert_rowid();

            // 5. 同步更新 rooms.meter_current
            queries::update_room_meters_tx(
                tx, req.room_id, req.water_reading, req.electric_reading,
            )?;

            Ok(MeterReadingResponse {
                success: true,
                message: "录入成功".to_string(),
                id: Some(id),
            })
        })
    }

    /// 批量录入抄表读数（逐条事务，单条失败不影响其他）
    pub fn batch_record_meter_readings<C: HasConnection>(
        &self,
        ctx: &C,
        req: &BatchMeterReadingRequest,
    ) -> Result<Vec<MeterReadingResponse>> {
        let mut results = Vec::with_capacity(req.readings.len());
        for reading in &req.readings {
            let result = self.record_meter_reading(ctx, reading)?;
            results.push(result);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::TestContext;
    use crate::db::test_helpers::{
        create_test_db_in_memory, insert_test_room, insert_test_meter_reading_for_month,
    };
    use std::ops::Deref;

    /// 新房间首次录入 → 成功
    #[test]
    fn test_record_meter_reading_success() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M001", "空房", 0);
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 100,
            electric_reading: 200,
            reading_date: "2026-04-10".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(result.success, "首次录入应成功: {}", result.message);
        assert!(result.id.is_some());
        assert_eq!(result.message, "录入成功");
    }

    /// 二次录入，读数 > 上期 → 成功
    #[test]
    fn test_record_meter_reading_increment() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M002", "在租", 300000);
        insert_test_meter_reading_for_month(&conn, room_id, 2026, 3, 50, 200, "2026-03-15");
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 80,   // > 50
            electric_reading: 350, // > 200
            reading_date: "2026-04-15".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(result.success, "读数递增应成功: {}", result.message);
    }

    /// 水表倒拨 → 拒绝
    #[test]
    fn test_record_meter_reading_water_rollback_rejected() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M003", "在租", 300000);
        insert_test_meter_reading_for_month(&conn, room_id, 2026, 3, 100, 200, "2026-03-15");
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 50,    // < 100，倒拨
            electric_reading: 300,
            reading_date: "2026-04-15".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(!result.success, "水表倒拨应被拒绝");
        assert!(result.message.contains("不能小于上期"), "message: {}", result.message);
    }

    /// 电表倒拨 → 拒绝
    #[test]
    fn test_record_meter_reading_electric_rollback_rejected() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M004", "在租", 300000);
        insert_test_meter_reading_for_month(&conn, room_id, 2026, 3, 50, 200, "2026-03-15");
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 80,
            electric_reading: 100, // < 200，倒拨
            reading_date: "2026-04-15".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(!result.success, "电表倒拨应被拒绝");
        assert!(result.message.contains("不能小于上期"), "message: {}", result.message);
    }

    /// 同房间同月重复录入 → 拒绝（幂等）
    #[test]
    fn test_record_meter_reading_idempotent() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M005", "在租", 300000);
        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 50, 200, "2026-04-15");
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 80,
            electric_reading: 300,
            reading_date: "2026-04-16".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(!result.success, "重复录入应被拒绝");
        assert!(result.message.contains("读数已录入"), "message: {}", result.message);
    }

    /// 录入后 rooms.meter_current 已更新
    #[test]
    fn test_record_meter_reading_updates_room_meters() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M006", "在租", 300000);
        // 初始值 0
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 123,
            electric_reading: 456,
            reading_date: "2026-04-15".to_string(),
            operator: None,
            is_replacement: false,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();
        assert!(result.success);

        // 验证 rooms.meter_current 已更新
        let room = queries::get_room_by_id(ctx.get_conn().unwrap().deref(), room_id).unwrap().unwrap();
        assert_eq!(room.water_meter_current, 123);
        assert_eq!(room.electric_meter_current, 456);
    }

    /// 批量3条，第2条倒拨，1和3成功，2失败
    #[test]
    fn test_batch_record_partial_success() {
        let conn = create_test_db_in_memory();
        let room1 = insert_test_room(&conn, "M011", "在租", 300000);
        let room2 = insert_test_room(&conn, "M012", "在租", 300000);
        let room3 = insert_test_room(&conn, "M013", "在租", 300000);

        // room2 需要先有上期读数（以便产生倒拨错误）
        insert_test_meter_reading_for_month(&conn, room2, 2026, 3, 100, 200, "2026-03-15");

        let ctx = TestContext::new(conn);

        let req = BatchMeterReadingRequest {
            readings: vec![
                // 第1条：room1 首次录入 → 成功
                MeterReadingRequest {
                    room_id: room1,
                    year: 2026,
                    month: 4,
                    water_reading: 50,
                    electric_reading: 100,
                    reading_date: "2026-04-15".to_string(),
                    operator: None,
                    is_replacement: false,
                },
                // 第2条：room2 水表倒拨 → 失败
                MeterReadingRequest {
                    room_id: room2,
                    year: 2026,
                    month: 4,
                    water_reading: 50, // < 100
                    electric_reading: 300,
                    reading_date: "2026-04-15".to_string(),
                    operator: None,
                    is_replacement: false,
                },
                // 第3条：room3 首次录入 → 成功
                MeterReadingRequest {
                    room_id: room3,
                    year: 2026,
                    month: 4,
                    water_reading: 80,
                    electric_reading: 150,
                    reading_date: "2026-04-15".to_string(),
                    operator: None,
                    is_replacement: false,
                },
            ],
        };

        let service = MeterReadingService;
        let results = service.batch_record_meter_readings(&ctx, &req).unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].success, "第1条应成功: {}", results[0].message);
        assert!(!results[1].success, "第2条应失败: {}", results[1].message);
        assert!(results[2].success, "第3条应成功: {}", results[2].message);
    }

    /// 换表场景：is_replacement=true，读数 < 上期 → 允许
    #[test]
    fn test_record_meter_reading_replacement_allows_lower() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "M007", "在租", 300000);
        insert_test_meter_reading_for_month(&conn, room_id, 2026, 3, 500, 1000, "2026-03-15");
        let ctx = TestContext::new(conn);

        let req = MeterReadingRequest {
            room_id,
            year: 2026,
            month: 4,
            water_reading: 10,    // < 500，但换表
            electric_reading: 20, // < 1000，但换表
            reading_date: "2026-04-15".to_string(),
            operator: None,
            is_replacement: true,
        };

        let service = MeterReadingService;
        let result = service.record_meter_reading(&ctx, &req).unwrap();

        assert!(result.success, "换表录入应成功: {}", result.message);
    }
}