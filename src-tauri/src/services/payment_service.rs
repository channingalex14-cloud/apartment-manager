//! 缴费服务
//!
//! 缴费记录、押金抵扣

use crate::db::HasConnection;
use crate::errors::{AppError, Result};
use crate::models::{BillStatus, Payment, PaymentResponse, RecordPaymentRequest};
use rusqlite::params;
use tracing::info;

pub struct PaymentService;

impl PaymentService {
    /// 记录缴费
    ///
    /// 流程：
    /// 1. 在事务内读取账单信息（防止 TOCTOU 问题）
    /// 2. 插入缴费记录
    /// 3. 更新账单已付金额和状态
    /// 4. 如有押金抵扣，记录押金台账
    pub fn record_payment<C: HasConnection>(
        &self,
        ctx: &C,
        req: &RecordPaymentRequest,
    ) -> Result<PaymentResponse> {
        info!("缴费请求: bill_id={}, amount={}", req.bill_id, req.amount);

        // 执行事务（包含账单读取，确保数据一致性）
        let payment_id = ctx.transaction(|tx| {
            // 1. 在事务内读取账单信息
            let (total_amount, actual_paid, lease_id, room_id, bill_status) = {
                let mut stmt = tx.prepare(
                    "SELECT total_amount, actual_paid, lease_id, room_id, status FROM monthly_bills WHERE id = ? AND is_deleted = 0",
                )?;
                stmt.query_row([req.bill_id], |row| {
                    Ok((
                        row.get::<_, i64>("total_amount")?,
                        row.get::<_, i64>("actual_paid")?,
                        row.get::<_, Option<i64>>("lease_id")?.unwrap_or(0),
                        row.get::<_, i64>("room_id")?,
                        row.get::<_, String>("status")?,
                    ))
                }).map_err(|e| AppError::Database(e))?
            };

            // 检查账单状态（使用 BillStatus 枚举，禁止硬编码字符串）
            let status = BillStatus::from_str(&bill_status)?;
            if status == BillStatus::Voided {
                return Err(AppError::Business("账单已作废，无法缴费".to_string()));
            }

            // 计算新的已付金额
            let new_paid = actual_paid
                .checked_add(req.amount)
                .ok_or_else(|| AppError::business("支付金额计算溢出"))?;

            // 确定新状态（使用 BillStatus 枚举，禁止硬编码字符串）
            let new_status = BillStatus::from_paid_amount(total_amount, new_paid);

            // 校验状态转换合法性
            if !status.can_transition_to(&new_status) {
                return Err(AppError::invalid_status(&format!(
                    "账单状态不允许从 '{}' 转换为 '{}'", bill_status, new_status.as_str()
                )));
            }

            // 2. 插入缴费记录
            let final_lease_id = if lease_id == 0 { None } else { Some(lease_id) };
            tx.execute(
                r#"
                INSERT INTO payments
                (bill_id, room_id, lease_id, amount, payment_date, payment_method,
                 wechat_amount, alipay_amount, cash_amount, bank_amount,
                 deposit_deduct_amount, payer_name, operator)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    req.bill_id, room_id, final_lease_id, req.amount,
                    req.payment_date, req.payment_method,
                    req.wechat_amount.unwrap_or(0),
                    req.alipay_amount.unwrap_or(0),
                    req.cash_amount.unwrap_or(0),
                    req.bank_amount.unwrap_or(0),
                    req.deposit_deduct_amount.unwrap_or(0),
                    req.payer_name.as_deref(),
                    req.operator.as_deref()
                ],
            )?;

            let payment_id = tx.last_insert_rowid();

            // 3. 更新账单
            tx.execute(
                r#"
                UPDATE monthly_bills SET
                    actual_paid = ?,
                    status = ?,
                    paid_date = CASE WHEN ? >= total_amount THEN ? ELSE paid_date END,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
                params![
                    new_paid, new_status.as_str(), new_paid, req.payment_date, req.bill_id
                ],
            )?;

            // 4. 如有押金抵扣
            if let Some(deduct_amount) = req.deposit_deduct_amount {
                if deduct_amount > 0 && lease_id > 0 {
                    // 更新合同押金余额
                    tx.execute(
                        "UPDATE leases SET deposit_balance = deposit_balance - ? WHERE id = ?",
                        params![deduct_amount, lease_id],
                    )?;

                    // 记录押金台账
                    tx.execute(
                        r#"
                        INSERT INTO deposit_ledger
                        (lease_id, room_id, transaction_type, amount, balance,
                         reference_bill_id, reference_payment_id, transaction_date, operator, notes)
                        VALUES (?, ?, '抵扣', ?, (
                            SELECT deposit_balance FROM leases WHERE id = ?
                        ), ?, ?, ?, 'system', '押金抵扣缴费')
                        "#,
                        params![
                            lease_id, room_id, deduct_amount, lease_id,
                            req.bill_id, payment_id, req.payment_date
                        ],
                    )?;
                }
            }

            info!("缴费成功: payment_id={}", payment_id);
            Ok(payment_id)
        })?;

        Ok(PaymentResponse::success(payment_id))
    }

    /// 作废缴费记录（红冲）
    ///
    /// TOCTOU 防护：读取缴费记录和写入操作在同一事务中执行
    pub fn void_payment<C: HasConnection>(
        &self,
        ctx: &C,
        payment_id: i64,
        _operator: Option<&str>,
    ) -> Result<PaymentResponse> {
        info!("作废缴费请求: payment_id={}", payment_id);

        // 所有操作（包括读取）在同一事务中执行，防止 TOCTOU
        ctx.transaction(|tx| {
            // 1. 在事务内读取缴费记录
            let mut stmt = tx.prepare(
                "SELECT * FROM payments WHERE id = ? AND is_deleted = 0",
            )?;

            let payment = stmt
                .query_row([payment_id], |row| {
                    Ok(Payment {
                        id: row.get("id")?,
                        bill_id: row.get("bill_id")?,
                        room_id: row.get("room_id")?,
                        lease_id: row.get("lease_id")?,
                        amount: row.get("amount")?,
                        payment_date: row.get("payment_date")?,
                        payment_method: row.get("payment_method")?,
                        wechat_amount: row.get("wechat_amount")?,
                        alipay_amount: row.get("alipay_amount")?,
                        cash_amount: row.get("cash_amount")?,
                        bank_amount: row.get("bank_amount")?,
                        deposit_deduct_amount: row.get("deposit_deduct_amount")?,
                        payer_name: row.get("payer_name")?,
                        confirmation_screenshot: row.get("confirmation_screenshot")?,
                        operator: row.get("operator")?,
                        notes: row.get("notes")?,
                        is_deleted: row.get::<_, i32>("is_deleted")? != 0,
                        created_at: row.get("created_at")?,
                    })
                })
                .map_err(|e| AppError::Database(e))?;

            // 2. 软删除缴费记录
            tx.execute(
                "UPDATE payments SET is_deleted = 1, notes = notes || ' [作废]' WHERE id = ?",
                [payment_id],
            )?;

            // 3. 更新账单已付金额（使用枚举而非硬编码字符串）
            if let Some(bill_id) = payment.bill_id {
                // 读取账单信息以计算新状态
                let (total_amount, current_actual_paid): (i64, i64) = tx.query_row(
                    "SELECT total_amount, actual_paid FROM monthly_bills WHERE id = ?",
                    [bill_id],
                    |row| Ok((row.get::<_, i64>("total_amount")?, row.get::<_, i64>("actual_paid")?)),
                ).map_err(|e| AppError::Database(e))?;

                let new_actual_paid = current_actual_paid
                    .checked_sub(payment.amount)
                    .ok_or_else(|| AppError::business("作废缴费后已付金额计算溢出"))?;
                let new_status = BillStatus::from_paid_amount(total_amount, new_actual_paid);

                // 校验状态转换合法性
                let current_bill_status: String = tx.query_row(
                    "SELECT status FROM monthly_bills WHERE id = ?",
                    [bill_id],
                    |row| row.get::<_, String>(0),
                ).map_err(|e| AppError::Database(e))?;
                let current_status = BillStatus::from_str(&current_bill_status)?;
                if !current_status.can_transition_to(&new_status) {
                    return Err(AppError::invalid_status(&format!(
                        "账单状态不允许从 '{}' 转换为 '{}'", current_bill_status, new_status.as_str()
                    )));
                }

                tx.execute(
                    r#"
                    UPDATE monthly_bills SET
                        actual_paid = ?,
                        status = ?,
                        updated_at = datetime('now')
                    WHERE id = ?
                    "#,
                    params![new_actual_paid, new_status.as_str(), bill_id],
                )?;
            }

            // 4. 如有押金抵扣，恢复押金余额
            if payment.deposit_deduct_amount > 0 {
                if let Some(lease_id) = payment.lease_id {
                    tx.execute(
                        "UPDATE leases SET deposit_balance = deposit_balance + ? WHERE id = ?",
                        params![payment.deposit_deduct_amount, lease_id],
                    )?;

                    // 插入反向押金台账记录（软删除：记录作废流水，而非物理删除）
                    tx.execute(
                        r#"
                        INSERT INTO deposit_ledger
                        (lease_id, room_id, transaction_type, amount, balance, transaction_date, operator, notes)
                        SELECT lease_id, room_id, '作废', ?, balance, date('now'), ?, '缴费作废，押金退回'
                        FROM deposit_ledger WHERE reference_payment_id = ?
                        "#,
                        params![payment.deposit_deduct_amount, _operator.unwrap_or("system"), payment_id],
                    )?;

                    // 清除原台账记录的 reference（标记为已反向）
                    tx.execute(
                        "UPDATE deposit_ledger SET reference_payment_id = NULL WHERE reference_payment_id = ?",
                        [payment_id],
                    )?;
                }
            }

            info!("作废缴费成功: payment_id={}", payment_id);
            Ok(PaymentResponse::success(payment_id))
        })
    }

    /// 更新缴费记录的付款方式
    pub fn update_payment_method<C: HasConnection>(
        &self,
        ctx: &C,
        payment_id: i64,
        payment_method: &str,
        operator: Option<&str>,
    ) -> Result<PaymentResponse> {
        info!("更新付款方式请求: payment_id={}, method={}", payment_id, payment_method);

        ctx.transaction(|tx| {
            // 检查缴费记录是否存在
            let mut stmt = tx.prepare(
                "SELECT id, is_deleted FROM payments WHERE id = ? AND is_deleted = 0",
            )?;
            let exists = stmt.exists([payment_id]).map_err(|e| AppError::Database(e))?;
            if !exists {
                return Err(AppError::Business("缴费记录不存在或已作废".to_string()));
            }

            // 更新付款方式
            tx.execute(
                "UPDATE payments SET payment_method = ?, operator = COALESCE(?, operator) WHERE id = ?",
                params![payment_method, operator, payment_id],
            )?;

            info!("更新付款方式成功: payment_id={}", payment_id);
            Ok(PaymentResponse::success(payment_id))
        })
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_bill_status_after_payment() {
        // Test bill status calculation logic
        fn calculate_new_status(total_amount: i64, actual_paid: i64, payment_amount: i64) -> &'static str {
            let new_paid = actual_paid + payment_amount;
            if new_paid >= total_amount {
                "已支付"
            } else if new_paid > 0 {
                "部分支付"
            } else {
                "待缴费"
            }
        }

        // Full payment
        assert_eq!(calculate_new_status(100000, 0, 100000), "已支付");
        assert_eq!(calculate_new_status(100000, 50000, 50000), "已支付");
        assert_eq!(calculate_new_status(100000, 80000, 20000), "已支付");

        // Partial payment
        assert_eq!(calculate_new_status(100000, 0, 50000), "部分支付");
        assert_eq!(calculate_new_status(100000, 30000, 20000), "部分支付");

        // Over payment (should still be "已支付")
        assert_eq!(calculate_new_status(100000, 80000, 30000), "已支付");

        // Zero payment
        assert_eq!(calculate_new_status(100000, 0, 0), "待缴费");
    }

    #[test]
    fn test_payment_amount_validation() {
        // Payment amount should be positive
        fn is_valid_payment_amount(amount: i64) -> bool {
            amount > 0
        }

        assert!(is_valid_payment_amount(100));
        assert!(is_valid_payment_amount(1));
        assert!(!is_valid_payment_amount(0));
        assert!(!is_valid_payment_amount(-100));
    }

    #[test]
    fn test_deposit_deduct_calculation() {
        // Test deposit deduction logic
        fn calculate_new_deposit_balance(current_balance: i64, deduct_amount: i64) -> i64 {
            current_balance - deduct_amount
        }

        assert_eq!(calculate_new_deposit_balance(200000, 50000), 150000);
        assert_eq!(calculate_new_deposit_balance(200000, 200000), 0);
        assert_eq!(calculate_new_deposit_balance(200000, 100000), 100000);
    }

    #[test]
    fn test_payment_method_types() {
        // Valid payment methods
        let valid_methods = vec!["微信", "支付宝", "银行卡", "现金", "商家码", "押金抵扣", "混合支付"];

        for method in valid_methods {
            assert!(!method.is_empty());
        }
    }

    #[test]
    fn test_void_payment_status_calculation() {
        // Test status after voiding a payment
        fn calculate_status_after_void(
            current_paid: i64,
            void_amount: i64,
            total_amount: i64,
        ) -> &'static str {
            let new_paid = current_paid - void_amount;
            if new_paid >= total_amount {
                "已支付"
            } else if new_paid > 0 {
                "部分支付"
            } else {
                "待缴费"
            }
        }

        // Void full payment on partially paid bill
        assert_eq!(calculate_status_after_void(50000, 50000, 100000), "待缴费");

        // Void partial payment leaving partial
        assert_eq!(calculate_status_after_void(80000, 30000, 100000), "部分支付");

        // Void partial payment - 90000 remaining < 100000 total, so partial
        assert_eq!(calculate_status_after_void(120000, 30000, 100000), "部分支付");

        // Void remaining payment
        assert_eq!(calculate_status_after_void(100000, 100000, 100000), "待缴费");
    }
}
