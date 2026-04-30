use crate::db::queries;
use crate::errors::{AppError, Result};
use crate::models::RoomStatus;
use rusqlite::Transaction;

pub struct RoomService;

impl RoomService {
    pub fn update_room_status(
        tx: &Transaction,
        room_id: i64,
        new_status: RoomStatus,
        operator: Option<&str>,
        notes: Option<&str>,
    ) -> Result<i64> {
        let room = queries::get_room_by_id_tx(tx, room_id)?
            .ok_or_else(|| AppError::not_found("房间", room_id))?;
        let previous_status = room.status;

        if previous_status == new_status {
            return Ok(0);
        }

        if !previous_status.allows_manual_transition_to(new_status) {
            return Err(AppError::invalid_status(&format!(
                "房间状态 '{}' 不允许直接修改为 '{}'。在租/新租/违约状态必须通过退房或违约处理流程变更。",
                previous_status, new_status
            )));
        }

        let count = tx.execute(
            "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_status, room_id],
        )?;

        tx.execute(
            r#"
            INSERT INTO room_status_log
            (room_id, previous_status, new_status, trigger_type,
             tenant_id, tenant_name, change_date, effective_date, operator, notes)
            SELECT ?, ?, ?, ?, NULL, NULL, date('now'), date('now'), ?, ?
            WHERE 1 = 1
            "#,
            rusqlite::params![
                room_id,
                previous_status.as_str(),
                new_status.as_str(),
                "手动修改",
                operator.unwrap_or("system"),
                notes.unwrap_or("")
            ],
        )?;

        Ok(count as i64)
    }
}
