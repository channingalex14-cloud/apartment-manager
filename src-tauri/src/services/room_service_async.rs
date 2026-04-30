//! 房间服务（异步版本，V3.0.0）
//!
//! Phase 3 实现：使用 sqlx 异步查询

use sqlx::Sqlite;
use crate::db::connection_async::AppContextAsync;
use crate::errors::{AppError, Result};
use crate::models::{Room, RoomStatus};

pub struct RoomServiceAsync;

impl RoomServiceAsync {
    /// 更新房间状态（异步版本）
    ///
    /// 对应 legacy: `RoomService::update_room_status`
    pub async fn update_room_status(
        ctx: &AppContextAsync,
        room_id: i64,
        new_status: RoomStatus,
        operator: Option<&str>,
        notes: Option<&str>,
    ) -> Result<i64> {
        let new_status_clone = new_status;
        let operator_clone = operator.map(|s| s.to_string());
        let notes_clone = notes.map(|s| s.to_string());

        ctx.transaction(|tx| {
            async move {
                let room = Self::get_room_by_id(tx, room_id).await?
                    .ok_or_else(|| AppError::not_found("房间", room_id))?;
                let previous_status = room.status;

                if previous_status == new_status_clone {
                    return Ok(0);
                }

                if !previous_status.allows_manual_transition_to(new_status_clone) {
                    return Err(AppError::invalid_status(&format!(
                        "房间状态 '{}' 不允许直接修改为 '{}'。在租/新租/违约状态必须通过退房或违约处理流程变更。",
                        previous_status, new_status_clone
                    )));
                }

                let result = sqlx::query(
                    "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2"
                )
                .bind(new_status_clone.as_str())
                .bind(room_id)
                .execute(&mut **tx)
                .await
                .map_err(|e| AppError::Database(format!("更新房间状态失败: {}", e)))?;

                sqlx::query(
                    r#"
                    INSERT INTO room_status_log
                    (room_id, previous_status, new_status, trigger_type,
                     tenant_id, tenant_name, change_date, effective_date, operator, notes)
                    SELECT ?, ?, ?, ?, NULL, NULL, date('now'), date('now'), ?, ?
                    WHERE 1 = 1
                    "#
                )
                .bind(room_id)
                .bind(previous_status.as_str())
                .bind(new_status_clone.as_str())
                .bind("手动修改")
                .bind(operator_clone.unwrap_or_else(|| "system".to_string()))
                .bind(notes_clone.unwrap_or_default())
                .execute(&mut **tx)
                .await
                .map_err(|e| AppError::Database(format!("记录状态日志失败: {}", e)))?;

                Ok(result.rows_affected() as i64)
            }
        }).await
    }

    /// 查询房间（事务内）
    async fn get_room_by_id(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        room_id: i64,
    ) -> Result<Option<Room>> {
        let result = sqlx::query_as::<_, (i64, String, Option<i32>, String, String, String, i64, i64, i64, bool, i64, Option<String>, Option<String>)>(
            r#"SELECT id, room_number, floor, building, room_type, status,
                      base_rent, property_fee, deposit,
                      is_deleted, version, created_at, updated_at
               FROM rooms WHERE id = ? AND is_deleted = 0"#
        )
        .bind(room_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| AppError::Database(format!("查询房间失败: {}", e)))?;

        Ok(result.map(|(id, room_number, floor, building, room_type, status_str, base_rent, property_fee, deposit, is_deleted, version, created_at, updated_at)| {
            Room {
                id,
                room_number,
                floor,
                building,
                room_type,
                status: RoomStatus::from_str(&status_str).unwrap_or(RoomStatus::Vacant),
                base_rent,
                property_fee,
                deposit,
                water_meter_current: 0,
                electric_meter_current: 0,
                is_deleted,
                version,
                created_at,
                updated_at,
            }
        }))
    }
}
