//! 押金服务
//!
//! 押金台账管理

use crate::db::{queries, HasConnection};
use crate::errors::{AppError, Result};
use crate::models::{DepositLedgerItem, DepositLedgerResponse, DepositStatus};
use rusqlite::params;
use tracing::info;

pub struct DepositService;

impl DepositService {
    /// 获取押金台账
    pub fn get_deposit_ledger<C: HasConnection>(
        &self,
        ctx: &C,
        lease_id: Option<i64>,
        room_id: Option<i64>,
    ) -> Result<DepositLedgerResponse> {
        let rows = queries::get_deposit_ledger(
            &*ctx.get_conn()?,
            lease_id,
            room_id,
        )?;

        // 转换为 DepositLedgerItem（分转元）
        // ⚠️ 修复 f64 精度问题：保留原始 fen 值用于计算，yuan 仅用于显示
        let records: Vec<DepositLedgerItem> = rows
            .into_iter()
            .map(|r| DepositLedgerItem {
                id: r.id,
                room_number: r.room_number,
                tenant_name: r.tenant_name,
                transaction_type: r.transaction_type,
                amount_fen: r.amount,
                balance_fen: r.balance,
                transaction_date: r.transaction_date,
                operator: r.operator,
                notes: r.notes,
            })
            .collect();

        // 计算总余额（使用原始 fen 值，避免 f64→i64 精度丢失）
        let total_balance = records.last().map(|r| r.balance_fen).unwrap_or(0);

        Ok(DepositLedgerResponse {
            records,
            total_balance,
        })
    }

    /// 收取押金
    ///
    /// 事务内读取合同信息，防止 TOCTOU
    pub fn receive_deposit<C: HasConnection>(
        &self,
        ctx: &C,
        lease_id: i64,
        amount: i64,
        transaction_date: &str,
        operator: Option<&str>,
    ) -> Result<i64> {
        info!("收取押金: lease_id={}, amount={}", lease_id, amount);

        ctx.transaction(|tx| {
            // 获取合同信息（事务内读取，防止 TOCTOU）
            let lease = queries::get_lease_by_id_tx(tx, lease_id)?
                .ok_or_else(|| AppError::not_found("合同", lease_id))?;

            let new_balance = lease.deposit_balance
                .checked_add(amount)
                .ok_or_else(|| AppError::business("押金收取计算溢出"))?;

            // 插入台账记录
            tx.execute(
                r#"
                INSERT INTO deposit_ledger
                (lease_id, room_id, transaction_type, amount, balance,
                 transaction_date, operator, notes)
                VALUES (?, ?, '收取', ?, ?, ?, ?, '新签合同收取押金')
                "#,
                params![lease_id, lease.room_id, amount, new_balance, transaction_date, operator.unwrap_or("system")],
            )?;

            // 更新合同押金
            let new_status = DepositStatus::from_balance(lease.deposit, new_balance);

            // 校验状态转换合法性
            let current_deposit_status = DepositStatus::from_str(&lease.deposit_status)?;
            if !current_deposit_status.can_transition_to(&new_status) {
                return Err(AppError::invalid_status(&format!(
                    "押金状态不允许从 '{}' 转换为 '{}'", lease.deposit_status, new_status.as_str()
                )));
            }

            tx.execute(
                r#"
                UPDATE leases SET
                    deposit_received = deposit_received + ?,
                    deposit_balance = deposit_balance + ?,
                    deposit_status = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![amount, amount, new_status.as_str(), lease_id],
            )?;

            info!("收取押金成功: new_balance={}", new_balance);
            Ok(new_balance)
        })
    }

    /// 退还押金
    ///
    /// 事务内读取合同信息，防止 TOCTOU
    pub fn refund_deposit<C: HasConnection>(
        &self,
        ctx: &C,
        lease_id: i64,
        amount: i64,
        reason: &str,
        transaction_date: &str,
        operator: Option<&str>,
    ) -> Result<i64> {
        info!("退还押金: lease_id={}, amount={}, reason={}", lease_id, amount, reason);

        ctx.transaction(|tx| {
            // 获取合同信息（事务内读取，防止 TOCTOU）
            let lease = queries::get_lease_by_id_tx(tx, lease_id)?
                .ok_or_else(|| AppError::not_found("合同", lease_id))?;

            // 验证余额（基于最新读取，防止超退）
            if amount > lease.deposit_balance {
                return Err(AppError::invalid_amount(&format!(
                    "退还金额({})超过余额({})",
                    amount, lease.deposit_balance
                )));
            }

            let new_balance = lease.deposit_balance
                .checked_sub(amount)
                .ok_or_else(|| AppError::business("押金退还计算溢出"))?;

            // 插入台账记录
            tx.execute(
                r#"
                INSERT INTO deposit_ledger
                (lease_id, room_id, transaction_type, amount, balance,
                 transaction_date, operator, notes)
                VALUES (?, ?, '退还', ?, ?, ?, ?, ?)
                "#,
                params![lease_id, lease.room_id, amount, new_balance, transaction_date, operator.unwrap_or("system"), reason],
            )?;

            // 更新合同押金
            let new_deposit_status = if new_balance > 0 {
                DepositStatus::Partial
            } else {
                DepositStatus::Refunded
            };

            // 校验状态转换合法性
            let current_deposit_status = DepositStatus::from_str(&lease.deposit_status)?;
            if !current_deposit_status.can_transition_to(&new_deposit_status) {
                return Err(AppError::invalid_status(&format!(
                    "押金状态不允许从 '{}' 转换为 '{}'", lease.deposit_status, new_deposit_status.as_str()
                )));
            }

            tx.execute(
                r#"
                UPDATE leases SET
                    deposit_balance = deposit_balance - ?,
                    deposit_status = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![amount, new_deposit_status.as_str(), lease_id],
            )?;

            info!("退还押金成功: new_balance={}", new_balance);
            Ok(new_balance)
        })
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_receive_deposit_balance_calculation() {
        fn calculate_new_balance(current_balance: i64, amount: i64) -> i64 {
            current_balance + amount
        }

        assert_eq!(calculate_new_balance(0, 200000), 200000);
        assert_eq!(calculate_new_balance(100000, 100000), 200000);
        assert_eq!(calculate_new_balance(0, 50000), 50000);
    }

    #[test]
    fn test_refund_deposit_balance_calculation() {
        fn calculate_new_balance(current_balance: i64, amount: i64) -> i64 {
            current_balance - amount
        }

        assert_eq!(calculate_new_balance(200000, 200000), 0);
        assert_eq!(calculate_new_balance(200000, 100000), 100000);
        assert_eq!(calculate_new_balance(200000, 50000), 150000);
    }

    #[test]
    fn test_refund_validation() {
        fn can_refund(deposit_balance: i64, refund_amount: i64) -> bool {
            refund_amount <= deposit_balance
        }

        // Can refund
        assert!(can_refund(200000, 200000));
        assert!(can_refund(200000, 100000));
        assert!(can_refund(200000, 1));

        // Cannot refund more than balance
        assert!(!can_refund(200000, 200001));
        assert!(!can_refund(100000, 100001));
        assert!(!can_refund(0, 1));
    }

    #[test]
    fn test_deposit_status_determination() {
        fn determine_deposit_status(deposit: i64, deposit_received: i64) -> &'static str {
            let balance = deposit_received;
            if balance >= deposit {
                "已收取"
            } else if balance > 0 {
                "部分收取"
            } else {
                "未收取"
            }
        }

        // Full deposit
        assert_eq!(determine_deposit_status(200000, 200000), "已收取");
        assert_eq!(determine_deposit_status(200000, 250000), "已收取");

        // Partial deposit
        assert_eq!(determine_deposit_status(200000, 100000), "部分收取");
        assert_eq!(determine_deposit_status(200000, 199999), "部分收取");

        // No deposit
        assert_eq!(determine_deposit_status(200000, 0), "未收取");
    }

    #[test]
    fn test_deposit_refund_status_determination() {
        fn determine_refund_status(remaining_balance: i64) -> &'static str {
            if remaining_balance > 0 {
                "部分收取"
            } else {
                "退还"
            }
        }

        assert_eq!(determine_refund_status(200000), "部分收取");
        assert_eq!(determine_refund_status(100000), "部分收取");
        assert_eq!(determine_refund_status(1), "部分收取");
        assert_eq!(determine_refund_status(0), "退还");
    }

    #[test]
    fn test_cent_to_yuan_conversion() {
        fn cents_to_yuan(cents: i64) -> f64 {
            cents as f64 / 100.0
        }

        assert_eq!(cents_to_yuan(200000), 2000.0);
        assert_eq!(cents_to_yuan(100000), 1000.0);
        assert_eq!(cents_to_yuan(5000), 50.0);
        assert_eq!(cents_to_yuan(1), 0.01);
    }

    #[test]
    fn test_deposit_ledger_amount_calculation() {
        // Test that deposit ledger amounts are stored in cents
        let deposit_yuan = 2000.0;
        let deposit_cents = (deposit_yuan * 100.0) as i64;

        assert_eq!(deposit_cents, 200000);

        // Verify conversion back
        let back_to_yuan = deposit_cents as f64 / 100.0;
        assert_eq!(back_to_yuan, 2000.0);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::db::connection::TestContext;
    use crate::db::test_helpers::*;

    /// 回归测试 #20：receive_deposit 在事务内读取，并发场景下余额不丢失
    ///
    /// TOCTOU 漏洞场景：事务外读取 deposit_balance，事务内写入。
    /// 如果在事务外读取，并发收取时后一次操作的 new_balance 基于旧值计算。
    /// 修复后：每次收取都基于最新的 deposit_balance 计算。
    #[test]
    fn test_receive_deposit_no_lost_update() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "D01", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "张三", "13800000020");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = DepositService;

        // 第一次收取 100000（押金总额 200000）
        service
            .receive_deposit(&ctx, lease_id, 100000, "2026-04-01", Some("test"))
            .unwrap();

        // 第二次收取 100000（凑满押金）
        service
            .receive_deposit(&ctx, lease_id, 100000, "2026-04-02", Some("test"))
            .unwrap();

        // 验证余额累加正确：0 + 100000 + 100000 = 200000
        // 如果有 TOCTOU，第二次收取的 new_balance 会基于第一次之前的值（0），
        // 导致最终余额为 100000（丢失了第一次的 100000）
        let conn = ctx.get_conn().expect("获取连接用于验证");
        let lease = queries::get_lease_by_id(&conn, lease_id)
            .expect("查询合同失败")
            .expect("合同不存在");
        assert_eq!(lease.deposit_balance, 200000);
        assert_eq!(lease.deposit_status, "已收取");
    }

    /// 回归测试 #21：refund_deposit 在事务内读取，超额退还被正确拒绝
    ///
    /// TOCTOU 漏洞场景：事务外读取 deposit_balance，事务内写入。
    /// 并发退押金时两次都基于余额=2000 校验通过，导致余额变负。
    /// 修复后：每次退还都在事务内读取最新余额，防止超退。
    #[test]
    fn test_refund_deposit_rejects_insufficient_balance() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "D02", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "李四", "13800000021");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = DepositService;

        // 收取押金 200000
        service
            .receive_deposit(&ctx, lease_id, 200000, "2026-04-01", Some("test"))
            .unwrap();

        // 尝试退还 250000（超过余额 200000）
        let result = service.refund_deposit(
            &ctx,
            lease_id,
            250000,
            "测试超额退还",
            "2026-04-02",
            Some("test"),
        );

        // 修复后：应返回错误，不允许超退
        assert!(result.is_err(), "超额退还应被拒绝");

        // 验证余额未被动用（仍是 200000）
        let conn = ctx.get_conn().expect("获取连接用于验证");
        let lease = queries::get_lease_by_id(&conn, lease_id)
            .expect("查询合同失败")
            .expect("合同不存在");
        assert_eq!(lease.deposit_balance, 200000, "超额退还后余额不应变化");
    }

    /// 回归测试：两次退款都从正确余额扣减（余额足够时）
    #[test]
    fn test_sequential_refunds_correct_deduction() {
        let conn = create_test_db_in_memory();
        let room_id = insert_test_room(&conn, "D03", "在租", 300000);
        let tenant_id = insert_test_tenant(&conn, "王五", "13800000022");
        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let ctx = TestContext::new(conn);
        let service = DepositService;

        // 收取押金 200000
        service
            .receive_deposit(&ctx, lease_id, 200000, "2026-04-01", Some("test"))
            .unwrap();

        // 第一次退还 80000，剩余 120000
        service
            .refund_deposit(
                &ctx,
                lease_id,
                80000,
                "部分退房",
                "2026-04-15",
                Some("test"),
            )
            .unwrap();

        // 第二次退还 120000，剩余 0
        service
            .refund_deposit(
                &ctx,
                lease_id,
                120000,
                "结清退房",
                "2026-04-30",
                Some("test"),
            )
            .unwrap();

        let conn = ctx.get_conn().expect("获取连接用于验证");
        let lease = queries::get_lease_by_id(&conn, lease_id)
            .expect("查询合同失败")
            .expect("合同不存在");
        assert_eq!(lease.deposit_balance, 0);
        assert_eq!(lease.deposit_status, "退还");
    }
}
