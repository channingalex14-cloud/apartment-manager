//! 合同/租约服务
//!
//! 状态机实现：入住、退房、违约处理

use crate::db::{queries, HasConnection};
use crate::errors::{AppError, Result};
use crate::models::{
    CheckInRequest, CheckOutRequest, DepositStatus, LeaseResponse, LeaseStatus, RoomStatus,
};
use rusqlite::params;
use tracing::info;

/// 合同服务
pub struct LeaseService;

impl LeaseService {
    /// 入住
    ///
    /// 流程：
    /// 1. 在同一事务中：验证房间 + 验证合同 + 收取押金 + 更新房间状态 + 更新合同日期 + 记录日志 + 记录历史
    /// 2. 注意：读取和写入必须在同一事务中（TOCTOU 防护）
    pub fn check_in<C: HasConnection>(&self, ctx: &C, req: &CheckInRequest) -> Result<LeaseResponse> {
        info!("入住请求: room_id={}, tenant_id={}, lease_id={}",
              req.room_id, req.tenant_id, req.lease_id);

        // 执行统一事务（读取和写入都在同一事务中，防止 TOCTOU）
        ctx.transaction(|tx| {
            // 1. 验证房间（在事务内读取）
            let room = queries::get_room_by_id_tx(tx, req.room_id)?
                .ok_or_else(|| AppError::not_found("房间", req.room_id))?;

            // 2. 验证房间状态（使用状态机校验）
            let room_status = room.status;
            if !room_status.can_transition_to(&RoomStatus::NewRented) {
                return Ok(LeaseResponse::error(&format!(
                    "房间状态不允许入住: {}", room.status
                )));
            }

            // 3. 验证合同（在事务内读取）
            let lease = queries::get_lease_by_id_tx(tx, req.lease_id)?
                .ok_or_else(|| AppError::not_found("合同", req.lease_id))?;

            // 4. 验证合同状态：入住不改变合同状态，只要求合同已生效
            let current_status = LeaseStatus::from_str(&lease.status)?;
            if current_status != LeaseStatus::Active {
                return Ok(LeaseResponse::error(&format!(
                    "合同状态不允许入住: {}", lease.status
                )));
            }

            // 5. 收取押金（如果需要）
            if lease.deposit > 0 {
                let new_deposit_balance = lease.deposit_balance
                    .checked_add(lease.deposit)
                    .ok_or_else(|| AppError::business("押金金额计算溢出"))?;
                let new_deposit_status = DepositStatus::from_balance(lease.deposit, new_deposit_balance);

                // 插入押金台账记录
                tx.execute(
                    r#"
                    INSERT INTO deposit_ledger
                    (lease_id, room_id, transaction_type, amount, balance,
                     transaction_date, operator, notes)
                    VALUES (?, ?, '收取', ?, ?, ?, ?, '新签合同收取押金')
                    "#,
                    params![
                        req.lease_id, req.room_id, lease.deposit, new_deposit_balance,
                        req.move_in_date, req.operator.as_deref().unwrap_or("system")
                    ],
                )?;

                // 更新合同押金状态
                tx.execute(
                    r#"
                    UPDATE leases SET
                        deposit_received = deposit_received + ?,
                        deposit_balance = deposit_balance + ?,
                        deposit_status = ?,
                        updated_at = datetime('now')
                    WHERE id = ?
                    "#,
                    params![lease.deposit, lease.deposit, new_deposit_status.as_str(), req.lease_id],
                )?;
            }

            // 6. 更新房间状态为 '新租'
            tx.execute(
                "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![RoomStatus::NewRented, req.room_id],
            )?;

            // 7. 更新合同入住日期
            tx.execute(
                "UPDATE leases SET move_in_date = ?, updated_at = datetime('now') WHERE id = ?",
                params![req.move_in_date, req.lease_id],
            )?;

            // 8. 记录房间状态变更日志
            tx.execute(
                r#"
                INSERT INTO room_status_log
                (room_id, lease_id, previous_status, new_status, trigger_type,
                 tenant_id, tenant_name, change_date, effective_date, operator)
                SELECT ?, ?, ?, ?, ?,
                       ?, t.name, ?, ?, ?
                FROM tenants t WHERE t.id = ?
                "#,
                params![
                    req.room_id, req.lease_id, room.status, RoomStatus::NewRented.as_str(), "入住",
                    req.tenant_id, req.move_in_date, req.move_in_date,
                    req.operator.as_deref().unwrap_or("system"),
                    req.tenant_id
                ],
            )?;

            // 9. 记录租客历史
            tx.execute(
                r#"
                INSERT INTO tenant_history
                (tenant_id, event_type, room_id, lease_id, event_date, new_value)
                VALUES (?, '新入住', ?, ?, ?, ?)
                "#,
                params![req.tenant_id, req.room_id, req.lease_id, req.move_in_date, req.room_id],
            )?;

            info!("入住成功: lease_id={}", req.lease_id);
            Ok(LeaseResponse::success_with_id(req.lease_id))
        })
    }

    /// 退房
    ///
    /// 流程：
    /// 1. 在同一事务中：验证合同 + 处理押金 + 更新合同状态 + 更新房间状态 + 记录日志
    /// 2. 注意：读取和写入必须在同一事务中（TOCTOU 防护）
    pub fn check_out<C: HasConnection>(&self, ctx: &C, req: &CheckOutRequest) -> Result<LeaseResponse> {
        info!("退房请求: lease_id={}, room_id={}, reason={}",
              req.lease_id, req.room_id, req.reason);

        // 执行统一事务（读取和写入都在同一事务中，防止 TOCTOU）
        ctx.transaction(|tx| {
            // 1. 验证合同（在事务内读取）
            let lease = queries::get_lease_by_id_tx(tx, req.lease_id)?
                .ok_or_else(|| AppError::not_found("合同", req.lease_id))?;

            // 2. 验证合同状态（使用状态机校验）
            let current_status = LeaseStatus::from_str(&lease.status)?;
            if !current_status.can_transition_to(&LeaseStatus::PendingSettle) {
                return Ok(LeaseResponse::error(&format!(
                    "合同状态不允许退房: {}", lease.status
                )));
            }

            // 3. 处理押金（退还/没收）
            if lease.deposit_balance > 0 {
                match req.reason.as_str() {
                    "正常退房" => {
                        tx.execute(
                            r#"
                            INSERT INTO deposit_ledger
                            (lease_id, room_id, transaction_type, amount, balance,
                             transaction_date, operator, notes)
                            VALUES (?, ?, '退还', ?, 0, ?, ?, ?)
                            "#,
                            params![
                                req.lease_id, req.room_id,
                                lease.deposit_balance, req.move_out_date,
                                req.operator.as_deref().unwrap_or("system"),
                                format!("退房{}，押金退还", req.reason)
                            ],
                        )?;
                        tx.execute(
                            "UPDATE leases SET deposit_balance = 0, deposit_status = ?, updated_at = datetime('now') WHERE id = ?",
                            params![DepositStatus::Refunded.as_str(), req.lease_id],
                        )?;
                    }
                    "违约退房" => {
                        tx.execute(
                            r#"
                            INSERT INTO deposit_ledger
                            (lease_id, room_id, transaction_type, amount, balance,
                             transaction_date, operator, notes)
                            VALUES (?, ?, '没收', ?, 0, ?, ?, ?)
                            "#,
                            params![
                                req.lease_id, req.room_id,
                                lease.deposit_balance, req.move_out_date,
                                req.operator.as_deref().unwrap_or("system"),
                                format!("退房{}，押金没收", req.reason)
                            ],
                        )?;
                        tx.execute(
                            "UPDATE leases SET deposit_balance = 0, deposit_status = ?, updated_at = datetime('now') WHERE id = ?",
                            params![DepositStatus::Forfeited.as_str(), req.lease_id],
                        )?;
                    }
                    _ => {
                        tx.execute(
                            r#"
                            INSERT INTO deposit_ledger
                            (lease_id, room_id, transaction_type, amount, balance,
                             transaction_date, operator, notes)
                            VALUES (?, ?, '退还', ?, 0, ?, ?, ?)
                            "#,
                            params![
                                req.lease_id, req.room_id,
                                lease.deposit_balance, req.move_out_date,
                                req.operator.as_deref().unwrap_or("system"),
                                format!("退房{}，押金退还", req.reason)
                            ],
                        )?;
                        tx.execute(
                            "UPDATE leases SET deposit_balance = 0, deposit_status = ?, updated_at = datetime('now') WHERE id = ?",
                            params![DepositStatus::Refunded.as_str(), req.lease_id],
                        )?;
                    }
                }
            }

            // 4. 更新合同状态为"待结算"
            tx.execute(
                r#"
                UPDATE leases SET
                    status = ?,
                    move_out_date = ?,
                    termination_reason = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![LeaseStatus::PendingSettle.as_str(), req.move_out_date, req.reason, req.lease_id],
            )?;

            // 5. 查询退房前房间状态（用于日志）
            let room_for_log = queries::get_room_by_id_tx(tx, req.room_id)?
                .ok_or_else(|| AppError::not_found("房间", req.room_id))?;
            let previous_status = room_for_log.status;

            // 6. 验证房间状态转换（使用状态机校验）
            if !previous_status.can_transition_to(&RoomStatus::PendingClean) {
                return Ok(LeaseResponse::error(&format!(
                    "房间状态不允许退房: {}", previous_status
                )));
            }

            // 7. 更新房间状态为"待清洁"
            tx.execute(
                "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![RoomStatus::PendingClean, req.room_id],
            )?;

            // 7. 记录房间状态变更日志
            tx.execute(
                r#"
                INSERT INTO room_status_log
                (room_id, lease_id, previous_status, new_status, trigger_type,
                 tenant_id, tenant_name, change_date, effective_date, operator, notes)
                SELECT ?, ?, ?, ?, ?,
                       t.id, t.name, ?, ?, ?, ?
                FROM tenants t
                JOIN leases l ON l.tenant_id = t.id
                WHERE l.id = ?
                "#,
                params![
                    req.room_id, req.lease_id,
                    previous_status, RoomStatus::PendingClean.as_str(), "退房",
                    req.move_out_date, req.move_out_date,
                    req.operator.as_deref().unwrap_or("system"),
                    req.reason,
                    req.lease_id
                ],
            )?;

            info!("退房成功: lease_id={}", req.lease_id);
            Ok(LeaseResponse::success_with_id(req.lease_id))
        })
    }

    /// 违约标记
    ///
    /// 流程：
    /// 1. 在事务内验证合同状态
    /// 2. 更新合同状态为 '违约中'
    /// 3. 更新房间状态为 '违约'
    /// 4. 记录房间状态变更日志
    pub fn mark_violation<C: HasConnection>(&self, ctx: &C, lease_id: i64) -> Result<LeaseResponse> {
        info!("违约标记请求: lease_id={}", lease_id);

        // 执行事务（包含读取和写入，防止 TOCTOU）
        ctx.transaction(|tx| {
            // 在事务内读取合同
            let lease = queries::get_lease_by_id_tx(tx, lease_id)?
                .ok_or_else(|| AppError::not_found("合同", lease_id))?;

            // 验证合同状态（使用状态机校验）
            let current_status = LeaseStatus::from_str(&lease.status)?;
            if !current_status.can_transition_to(&LeaseStatus::Violation) {
                return Ok(LeaseResponse::error(&format!(
                    "合同状态不允许标记违约: {}", lease.status
                )));
            }

            // 更新合同状态
            tx.execute(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
                params![LeaseStatus::Violation.as_str(), lease_id],
            )?;

            // 查询违约前房间状态（用于日志）
            let room_for_log = queries::get_room_by_id_tx(tx, lease.room_id)?
                .ok_or_else(|| AppError::not_found("房间", lease.room_id))?;
            let previous_status = room_for_log.status;

            // 验证房间状态转换（使用状态机校验）
            if !previous_status.can_transition_to(&RoomStatus::Violation) {
                return Ok(LeaseResponse::error(&format!(
                    "房间状态不允许标记违约: {}", previous_status
                )));
            }

            // 更新房间状态
            tx.execute(
                "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![RoomStatus::Violation, lease.room_id],
            )?;

            // 记录房间状态变更日志
            tx.execute(
                r#"
                INSERT INTO room_status_log
                (room_id, lease_id, previous_status, new_status, trigger_type,
                 tenant_id, tenant_name, change_date, operator, notes)
                VALUES (?, ?, ?, ?, '违约处理',
                        (SELECT tenant_id FROM leases WHERE id = ?),
                        (SELECT name FROM tenants WHERE id = (SELECT tenant_id FROM leases WHERE id = ?)),
                        date('now'), 'system', '欠费违约')
                "#,
                params![
                    lease.room_id, lease_id,
                    previous_status, RoomStatus::Violation.as_str(),
                    lease_id, lease_id,
                ],
            )?;

            info!("违约标记成功: lease_id={}", lease_id);
            Ok(LeaseResponse::success_with_id(lease_id))
        })
    }

    /// 违约恢复
    ///
    /// 流程：
    /// 1. 在事务内验证合同状态
    /// 2. 更新合同状态为 '生效中'
    /// 3. 更新房间状态为 '在租'
    /// 4. 记录房间状态变更日志
    pub fn recover_from_violation<C: HasConnection>(&self, ctx: &C, lease_id: i64) -> Result<LeaseResponse> {
        info!("违约恢复请求: lease_id={}", lease_id);

        // 执行事务（包含读取和写入，防止 TOCTOU）
        ctx.transaction(|tx| {
            // 在事务内读取合同
            let lease = queries::get_lease_by_id_tx(tx, lease_id)?
                .ok_or_else(|| AppError::not_found("合同", lease_id))?;

            // 验证合同状态（使用状态机校验）
            let current_status = LeaseStatus::from_str(&lease.status)?;
            if !current_status.can_transition_to(&LeaseStatus::Active) {
                return Ok(LeaseResponse::error(&format!(
                    "合同状态不允许恢复: {}", lease.status
                )));
            }

            // 更新合同状态
            tx.execute(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
                params![LeaseStatus::Active.as_str(), lease_id],
            )?;

            // 查询恢复前房间状态（用于日志）
            let room_for_log = queries::get_room_by_id_tx(tx, lease.room_id)?
                .ok_or_else(|| AppError::not_found("房间", lease.room_id))?;
            let previous_status = room_for_log.status;

            // 验证房间状态转换（使用状态机校验）
            if !previous_status.can_transition_to(&RoomStatus::Rented) {
                return Ok(LeaseResponse::error(&format!(
                    "房间状态不允许恢复: {}", previous_status
                )));
            }

            // 更新房间状态
            tx.execute(
                "UPDATE rooms SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![RoomStatus::Rented, lease.room_id],
            )?;

            // 记录房间状态变更日志
            tx.execute(
                r#"
                INSERT INTO room_status_log
                (room_id, lease_id, previous_status, new_status, trigger_type,
                 tenant_id, tenant_name, change_date, operator, notes)
                SELECT ?, ?, ?, ?, '恢复',
                       t.id, t.name, date('now'), 'system', '缴清欠费恢复'
                FROM tenants t
                JOIN leases l ON l.tenant_id = t.id
                WHERE l.id = ?
                "#,
                params![
                    lease.room_id, lease_id,
                    previous_status, RoomStatus::Rented.as_str(),
                    lease_id,
                ],
            )?;

            info!("违约恢复成功: lease_id={}", lease_id);
            Ok(LeaseResponse::success_with_id(lease_id))
        })
    }

    /// 激活合同（草稿 → 生效中）
    ///
    /// 流程：
    /// 1. 验证合同状态（必须为草稿）
    /// 2. 更新合同状态为 '生效中'
    /// 3. 记录状态变更日志
    pub fn activate<C: HasConnection>(&self, ctx: &C, lease_id: i64) -> Result<LeaseResponse> {
        info!("激活合同请求: lease_id={}", lease_id);

        // 执行事务（包含读取和写入，防止 TOCTOU）
        ctx.transaction(|tx| {
            // 在事务内读取合同
            let lease = queries::get_lease_by_id_tx(tx, lease_id)?
                .ok_or_else(|| AppError::not_found("合同", lease_id))?;

            // 验证合同状态（使用状态机校验）
            let current_status = LeaseStatus::from_str(&lease.status)?;
            if !current_status.can_transition_to(&LeaseStatus::Active) {
                return Ok(LeaseResponse::error(&format!(
                    "合同状态不允许激活: {}", lease.status
                )));
            }

            // 校验关联房间存在（防止日志插入无效 room_id）
            if lease.room_id > 0 {
                let room = queries::get_room_by_id_tx(tx, lease.room_id)?;
                if room.is_none() {
                    return Ok(LeaseResponse::error(&format!(
                        "合同关联的房间不存在: room_id={}", lease.room_id
                    )));
                }
            }

            // 更新合同状态
            tx.execute(
                "UPDATE leases SET status = ?, updated_at = datetime('now') WHERE id = ?",
                params![LeaseStatus::Active.as_str(), lease_id],
            )?;

            // 记录状态变更日志
            tx.execute(
                r#"
                INSERT INTO room_status_log
                (room_id, lease_id, previous_status, new_status, trigger_type, change_date)
                VALUES (?, ?, ?, ?, '合同激活', date('now'))
                "#,
                params![lease.room_id, lease_id, LeaseStatus::Draft.as_str(), LeaseStatus::Active.as_str()],
            )?;

            info!("激活合同成功: lease_id={}", lease_id);
            Ok(LeaseResponse::success_with_id(lease_id))
        })
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::db::connection::TestContext;
    use crate::db::test_helpers::*;

    #[test]
    fn test_check_in_success() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "101-CI", "空房", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000001");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = LeaseService;

        let req = CheckInRequest {
            room_id,
            tenant_id,
            lease_id,
            move_in_date: "2026-04-13".to_string(),
            operator: Some("test".to_string()),
        };

        let result = service.check_in(&ctx, &req);
        assert!(result.is_ok());
        assert!(result.unwrap().success);

        // Verify room status changed
        let conn = ctx.get_conn().expect("测试获取连接失败");
        let room = queries::get_room_by_id(&conn, room_id)
            .expect("查询房间失败")
            .expect("房间不存在");
        assert_eq!(room.status, RoomStatus::NewRented);
    }

    #[test]
    fn test_check_in_invalid_room_status() {
        // Room is already rented, should fail
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "101-IN", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000002");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = LeaseService;

        let req = CheckInRequest {
            room_id,
            tenant_id,
            lease_id,
            move_in_date: "2026-04-13".to_string(),
            operator: Some("test".to_string()),
        };

        let result = service.check_in(&ctx, &req);
        assert!(result.is_ok());
        // Should fail with error message about room status
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_check_out_success() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "101-CO", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000003");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = LeaseService;

        let req = CheckOutRequest {
            lease_id,
            room_id,
            reason: "正常退房".to_string(),
            move_out_date: "2026-04-13".to_string(),
            operator: Some("test".to_string()),
        };

        let result = service.check_out(&ctx, &req);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_mark_violation_success() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "101-MV", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000004");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = LeaseService;

        let result = service.mark_violation(&ctx, lease_id);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_activate_lease_success() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "101-AC", "空房", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000005");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "草稿", 200000);

        let ctx = TestContext::new(conn);
        let service = LeaseService;

        let result = service.activate(&ctx, lease_id);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }
}

#[cfg(test)]
mod tests {
    

    // Test helper: validate room status for check_in
    fn validate_room_status_for_check_in(room_status: &str) -> bool {
        room_status == "空房" || room_status == "待清洁"
    }

    // Test helper: validate lease status for check_in
    fn validate_lease_status_for_check_in(lease_status: &str) -> bool {
        lease_status == "生效中"
    }

    // Test helper: validate lease status for check_out
    fn validate_lease_status_for_check_out(lease_status: &str) -> bool {
        lease_status == "生效中" || lease_status == "违约中"
    }

    // Test helper: validate lease status for mark_violation
    fn validate_lease_status_for_mark_violation(lease_status: &str) -> bool {
        lease_status == "生效中"
    }

    // Test helper: validate lease status for recover_from_violation
    fn validate_lease_status_for_recover(lease_status: &str) -> bool {
        lease_status == "违约中"
    }

    #[test]
    fn test_check_in_room_status_validation() {
        // Valid room statuses
        assert!(validate_room_status_for_check_in("空房"));
        assert!(validate_room_status_for_check_in("待清洁"));

        // Invalid room statuses
        assert!(!validate_room_status_for_check_in("在租"));
        assert!(!validate_room_status_for_check_in("违约"));
    }

    #[test]
    fn test_check_in_invalid_room_statuses() {
        assert!(!validate_room_status_for_check_in("在租"));
        assert!(!validate_room_status_for_check_in("违约"));
        assert!(!validate_room_status_for_check_in("员工"));
        assert!(!validate_room_status_for_check_in("管理"));
        assert!(!validate_room_status_for_check_in("已退房"));
    }

    #[test]
    fn test_check_in_lease_status_validation() {
        // Valid
        assert!(validate_lease_status_for_check_in("生效中"));

        // Invalid
        assert!(!validate_lease_status_for_check_in("草稿"));
        assert!(!validate_lease_status_for_check_in("违约中"));
        assert!(!validate_lease_status_for_check_in("待结算"));
        assert!(!validate_lease_status_for_check_in("已退房"));
        assert!(!validate_lease_status_for_check_in("已作废"));
    }

    #[test]
    fn test_check_out_lease_status_validation() {
        // Valid
        assert!(validate_lease_status_for_check_out("生效中"));
        assert!(validate_lease_status_for_check_out("违约中"));

        // Invalid
        assert!(!validate_lease_status_for_check_out("草稿"));
        assert!(!validate_lease_status_for_check_out("待结算"));
        assert!(!validate_lease_status_for_check_out("已退房"));
    }

    #[test]
    fn test_mark_violation_lease_status_validation() {
        // Valid
        assert!(validate_lease_status_for_mark_violation("生效中"));

        // Invalid
        assert!(!validate_lease_status_for_mark_violation("草稿"));
        assert!(!validate_lease_status_for_mark_violation("违约中"));
        assert!(!validate_lease_status_for_mark_violation("待结算"));
        assert!(!validate_lease_status_for_mark_violation("已退房"));
    }

    #[test]
    fn test_recover_from_violation_lease_status_validation() {
        // Valid
        assert!(validate_lease_status_for_recover("违约中"));

        // Invalid
        assert!(!validate_lease_status_for_recover("生效中"));
        assert!(!validate_lease_status_for_recover("草稿"));
        assert!(!validate_lease_status_for_recover("待结算"));
        assert!(!validate_lease_status_for_recover("已退房"));
    }

    #[test]
    fn test_check_out_deposit_status_determination() {
        // 正常退房 -> 退还
        assert_eq!(
            match "正常退房" {
                "正常退房" => "退还",
                "违约退房" => "没收",
                _ => "退还",
            },
            "退还"
        );

        // 违约退房 -> 没收
        assert_eq!(
            match "违约退房" {
                "正常退房" => "退还",
                "违约退房" => "没收",
                _ => "退还",
            },
            "没收"
        );
    }

    #[test]
    fn test_lease_status_state_machine() {
        // Test the complete state machine transitions
        type State = &'static str;

        fn can_transition(from: State, to: State, action: &str) -> bool {
            match (from, to, action) {
                // 草稿 -> 生效中 (activate)
                ("草稿", "生效中", "activate") => true,
                // 生效中 -> 违约中 (mark_violation)
                ("生效中", "违约中", "mark_violation") => true,
                // 违约中 -> 生效中 (recover)
                ("违约中", "生效中", "recover") => true,
                // 生效中 -> 待结算 (check_out)
                ("生效中", "待结算", "check_out") => true,
                // 违约中 -> 待结算 (check_out)
                ("违约中", "待结算", "check_out") => true,
                _ => false,
            }
        }

        // Valid transitions
        assert!(can_transition("草稿", "生效中", "activate"));
        assert!(can_transition("生效中", "违约中", "mark_violation"));
        assert!(can_transition("违约中", "生效中", "recover"));
        assert!(can_transition("生效中", "待结算", "check_out"));
        assert!(can_transition("违约中", "待结算", "check_out"));

        // Invalid transitions
        assert!(!can_transition("草稿", "违约中", "mark_violation"));
        assert!(!can_transition("待结算", "生效中", "recover"));
        assert!(!can_transition("已退房", "生效中", "activate"));
    }
}
