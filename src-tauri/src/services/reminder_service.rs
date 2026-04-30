//! 提醒服务
//!
//! 通知提醒管理

use crate::db::connection::HasConnection;
use crate::errors::{AppError, Result};
use crate::models::reminder::{
    CreateReminderRequest, Reminder, ReminderListResponse, ReminderResponse,
    UpdateReminderStatusRequest,
};
use rusqlite::params;
use tracing::info;

/// 提醒服务
pub struct ReminderService;

impl ReminderService {
    /// 创建提醒
    pub fn create_reminder<C: HasConnection>(
        &self,
        ctx: &C,
        req: &CreateReminderRequest,
    ) -> Result<ReminderResponse> {
        info!("创建提醒: type={}, title={}", req.reminder_type, req.title);

        let id = ctx.transaction(|tx| {
            tx.execute(
                r#"
                INSERT INTO reminders
                (reminder_type, room_id, lease_id, title, message, scheduled_date, is_sent, is_read)
                VALUES (?, ?, ?, ?, ?, ?, 0, 0)
                "#,
                params![
                    req.reminder_type,
                    req.room_id,
                    req.lease_id,
                    req.title,
                    req.message,
                    req.scheduled_date,
                ],
            )?;
            Ok(tx.last_insert_rowid())
        })?;

        info!("创建提醒成功: id={}", id);
        Ok(ReminderResponse {
            success: true,
            reminder_id: Some(id),
            message: None,
        })
    }

    /// 获取提醒列表
    pub fn list_reminders<C: HasConnection>(
        &self,
        ctx: &C,
        room_id: Option<i64>,
        is_read: Option<bool>,
    ) -> Result<ReminderListResponse> {
        ctx.transaction(|tx| {
            let sql = match (room_id, is_read) {
                (Some(_), Some(_)) => {
                    "SELECT * FROM reminders WHERE is_deleted = 0 AND room_id = ? AND is_read = ? ORDER BY scheduled_date DESC, created_at DESC"
                }
                (Some(_), None) => {
                    "SELECT * FROM reminders WHERE is_deleted = 0 AND room_id = ? ORDER BY scheduled_date DESC, created_at DESC"
                }
                (None, Some(_)) => {
                    "SELECT * FROM reminders WHERE is_deleted = 0 AND is_read = ? ORDER BY scheduled_date DESC, created_at DESC"
                }
                (None, None) => {
                    "SELECT * FROM reminders WHERE is_deleted = 0 ORDER BY scheduled_date DESC, created_at DESC"
                }
            };

            let mut stmt = tx.prepare(sql)?;

            let rows = match (room_id, is_read) {
                (Some(rid), Some(ir)) => stmt.query_map(params![rid, ir as i64], map_reminder)?,
                (Some(rid), None) => stmt.query_map(params![rid], map_reminder)?,
                (None, Some(ir)) => stmt.query_map(params![ir as i64], map_reminder)?,
                (None, None) => stmt.query_map([], map_reminder)?,
            };

            let mut reminders = Vec::new();
            for row in rows {
                reminders.push(row?);
            }

            Ok(ReminderListResponse {
                success: true,
                data: reminders,
                message: None,
            })
        })
    }

    /// 获取待发送的提醒
    pub fn get_pending_reminders<C: HasConnection>(
        &self,
        ctx: &C,
    ) -> Result<ReminderListResponse> {
        ctx.transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT * FROM reminders WHERE is_deleted = 0 AND is_sent = 0 AND is_read = 0 ORDER BY scheduled_date ASC",
            )?;

            let rows = stmt.query_map([], map_reminder)?;

            let mut reminders = Vec::new();
            for row in rows {
                reminders.push(row?);
            }

            Ok(ReminderListResponse {
                success: true,
                data: reminders,
                message: None,
            })
        })
    }

    /// 更新提醒状态
    /// 修复：使用事务保证数据一致性
    pub fn update_reminder_status<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
        req: &UpdateReminderStatusRequest,
    ) -> Result<ReminderResponse> {
        let mut updates = Vec::new();

        if req.is_sent.is_some() {
            updates.push("is_sent = ?");
        }
        if req.is_read.is_some() {
            updates.push("is_read = ?");
        }

        if updates.is_empty() {
            return Ok(ReminderResponse {
                success: false,
                reminder_id: None,
                message: Some("没有需要更新的字段".to_string()),
            });
        }

        let sql = format!(
            "UPDATE reminders SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(is_sent) = req.is_sent {
            params_vec.push(Box::new(is_sent as i64));
        }
        if let Some(is_read) = req.is_read {
            params_vec.push(Box::new(is_read as i64));
        }
        params_vec.push(Box::new(id));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        // 修复：使用事务包装更新操作
        ctx.transaction(|tx| {
            let affected = tx.execute(&sql, params_refs.as_slice())
                .map_err(|e| AppError::Database(e))?;

            if affected > 0 {
                Ok(ReminderResponse {
                    success: true,
                    reminder_id: Some(id),
                    message: None,
                })
            } else {
                Ok(ReminderResponse {
                    success: false,
                    reminder_id: None,
                    message: Some(format!("提醒不存在: id={}", id)),
                })
            }
        })
    }

    /// 标记提醒为已发送
    pub fn mark_as_sent<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
    ) -> Result<ReminderResponse> {
        ctx.transaction(|tx| {
            let affected = tx.execute(
                "UPDATE reminders SET is_sent = 1, reminded_at = datetime('now') WHERE id = ?",
                [id],
            ).map_err(|e| AppError::Database(e))?;

            if affected > 0 {
                Ok(ReminderResponse {
                    success: true,
                    reminder_id: Some(id),
                    message: None,
                })
            } else {
                Ok(ReminderResponse {
                    success: false,
                    reminder_id: None,
                    message: Some(format!("提醒不存在: id={}", id)),
                })
            }
        })
    }

    /// 标记提醒为已读
    pub fn mark_as_read<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
    ) -> Result<ReminderResponse> {
        ctx.transaction(|tx| {
            let affected = tx.execute(
                "UPDATE reminders SET is_read = 1 WHERE id = ?",
                [id],
            ).map_err(|e| AppError::Database(e))?;

            if affected > 0 {
                Ok(ReminderResponse {
                    success: true,
                    reminder_id: Some(id),
                    message: None,
                })
            } else {
                Ok(ReminderResponse {
                    success: false,
                    reminder_id: None,
                    message: Some(format!("提醒不存在: id={}", id)),
                })
            }
        })
    }

    /// 删除提醒
    /// 修复：改为软删除，保留数据可追溯性
    pub fn delete_reminder<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
    ) -> Result<ReminderResponse> {
        // 使用事务包装软删除操作
        ctx.transaction(|tx| {
            let affected = tx.execute(
                "UPDATE reminders SET is_deleted = 1 WHERE id = ? AND is_deleted = 0",
                [id],
            ).map_err(|e| AppError::Database(e))?;

            if affected > 0 {
                Ok(ReminderResponse {
                    success: true,
                    reminder_id: Some(id),
                    message: None,
                })
            } else {
                Ok(ReminderResponse {
                    success: false,
                    reminder_id: None,
                    message: Some(format!("提醒不存在: id={}", id)),
                })
            }
        })
    }
}

/// 映射数据库行为 Reminder
fn map_reminder(row: &rusqlite::Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get("id")?,
        reminder_type: row.get("reminder_type")?,
        room_id: row.get("room_id")?,
        lease_id: row.get("lease_id")?,
        title: row.get("title")?,
        message: row.get("message")?,
        scheduled_date: row.get("scheduled_date")?,
        reminded_at: row.get("reminded_at")?,
        is_sent: row.get::<_, i64>("is_sent")? != 0,
        is_read: row.get::<_, i64>("is_read")? != 0,
        created_at: row.get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_reminder_type_list() {
        let types = vec![
            "租金到期",
            "合同到期",
            "退房提醒",
            "维护提醒",
        ];
        assert_eq!(types.len(), 4);
    }
}
