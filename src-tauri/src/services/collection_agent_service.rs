//! 租务 AI Agent 服务
//!
//! 核心逻辑：扫描欠租 -> 分级 -> 生成催租消息 -> 创建系统提醒 -> 可选违约标记

use chrono::Local;
use tracing::info;

use crate::db::connection::HasConnection;
use crate::db::queries;
use crate::errors::Result;
use crate::models::collection_agent::{
    CollectionAgentReport, EscalationLevel, OverdueBill, ReminderDraft,
    RunCollectionAgentRequest,
};

/// 租务 AI Agent 服务
pub struct CollectionAgentService;

impl CollectionAgentService {
    /// 将分转换为元（带格式）
    fn fen_to_yuan_str(fen: i64) -> String {
        let yuan = fen as f64 / 100.0;
        format!("{:.2}", yuan)
    }

    /// 计算逾期天数
    fn calculate_overdue_days(due_date_str: &Option<String>) -> i64 {
        let Some(due_str) = due_date_str else {
            return 0;
        };
        let due = match chrono::NaiveDate::parse_from_str(due_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return 0,
        };
        let today = Local::now().date_naive();
        let diff = today.signed_duration_since(due).num_days();
        diff
    }

    /// 生成催租消息草稿
    fn generate_reminder_draft(&self, bill: &OverdueBill) -> ReminderDraft {
        let tenant_name = bill.tenant_name.as_deref().unwrap_or("租客");
        let room_full = format!("{}{}室", bill.building, bill.room_number);
        let amount_str = Self::fen_to_yuan_str(bill.unpaid_amount);
        let today_str = Local::now().format("%Y年%m月%d日").to_string();

        let (title, message_body) = match bill.escalation_level {
            EscalationLevel::Mild => (
                format!("【温馨提示】{} {}月账单待缴费", room_full, bill.year_month),
                format!(
                    "您好 {name}，您{room}{month}月的账单（合计{amount}元）尚未支付。\
                     请尽快完成缴费，以免影响您的正常租住。\
                     如有疑问请联系公寓管理方。",
                    name = tenant_name,
                    room = room_full,
                    month = bill.year_month,
                    amount = amount_str
                ),
            ),
            EscalationLevel::Moderate => (
                format!("【催款通知】{} {}月账单逾期{}天", room_full, bill.year_month, bill.overdue_days),
                format!(
                    "您好 {name}，您{room}{month}月的账单（合计{amount}元）已逾期{days}天。\
                     请尽快缴纳，以免产生滞纳金并影响您的个人信用记录。\
                     如有特殊情况请及时与管理方联系。",
                    name = tenant_name,
                    room = room_full,
                    month = bill.year_month,
                    amount = amount_str,
                    days = bill.overdue_days
                ),
            ),
            EscalationLevel::Severe => (
                format!("【紧急催款】{} {}月账单逾期{}天", room_full, bill.year_month, bill.overdue_days),
                format!(
                    "您好 {name}，您{room}{month}月的账单（合计{amount}元）已逾期{days}天。\
                     逾期时间较长，请立即处理。\
                     如仍未在合理时间内完成缴费，管理方将按合同约定采取进一步措施。\
                     请尽快联系公寓管理方。",
                    name = tenant_name,
                    room = room_full,
                    month = bill.year_month,
                    amount = amount_str,
                    days = bill.overdue_days
                ),
            ),
            EscalationLevel::None => (
                format!("【待处理】{} {}月账单", room_full, bill.year_month),
                format!("您好 {}，您{} {}月的账单待处理。", tenant_name, room_full, bill.year_month),
            ),
        };

        ReminderDraft {
            bill_id: bill.bill_id,
            room_id: bill.room_id,
            lease_id: bill.lease_id,
            room_number: bill.room_number.clone(),
            building: bill.building.clone(),
            tenant_name: bill.tenant_name.clone(),
            tenant_phone: bill.tenant_phone.clone(),
            year_month: bill.year_month.clone(),
            unpaid_amount: bill.unpaid_amount,
            overdue_days: bill.overdue_days,
            escalation_level: bill.escalation_level,
            title,
            message: format!("({}) {}", today_str, message_body),
        }
    }

    /// 检查是否需要创建提醒（防打扰逻辑）
    fn should_create_reminder(&self, tx: &rusqlite::Transaction, bill: &OverdueBill, min_days: i64) -> Result<bool> {
        if bill.escalation_level == EscalationLevel::None {
            return Ok(false);
        }
        let last_at = queries::get_bill_last_reminder_at(tx, bill.bill_id)?;
        if let Some(last) = last_at {
            // 防打扰：同一级别 3 天内不重复催
            let last_date = match chrono::NaiveDate::parse_from_str(&last[..10], "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => return Ok(true),
            };
            let today = Local::now().date_naive();
            let days_since = today.signed_duration_since(last_date).num_days();
            if days_since < min_days {
                info!(
                    "[CollectionAgent] bill #{} 在 {} 天内已催过，跳过",
                    bill.bill_id, days_since
                );
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 构建报告
    fn build_report(
        &self,
        bills: &[OverdueBill],
        drafts: &[ReminderDraft],
        reminders_created: i32,
        rooms_marked: i32,
        executed: bool,
    ) -> CollectionAgentReport {
        let scanned = bills.len() as i32;
        let overdue = bills.iter().filter(|b| b.overdue_days > 0).count() as i32;
        let mild = bills.iter().filter(|b| b.escalation_level == EscalationLevel::Mild).count() as i32;
        let moderate = bills.iter().filter(|b| b.escalation_level == EscalationLevel::Moderate).count() as i32;
        let severe = bills.iter().filter(|b| b.escalation_level == EscalationLevel::Severe).count() as i32;
        let total_unpaid: i64 = bills.iter().map(|b| b.unpaid_amount).sum();

        CollectionAgentReport {
            scanned_count: scanned,
            overdue_count: overdue,
            mild_count: mild,
            moderate_count: moderate,
            severe_count: severe,
            reminders_created,
            rooms_marked_violation: rooms_marked,
            total_unpaid,
            drafts: drafts.to_vec(),
            executed,
        }
    }

    /// 获取配置布尔值（事务内）
    fn get_config_bool(&self, tx: &rusqlite::Transaction, key: &str, default: bool) -> Result<bool> {
        let val = queries::get_system_config_value_tx(tx, key)?;
        Ok(val.as_deref().map(|v| v == "true" || v == "1").unwrap_or(default))
    }

    /// 获取配置整数值
    fn get_config_i64(&self, tx: &rusqlite::Transaction, key: &str, default: i64) -> Result<i64> {
        let val = queries::get_system_config_value_tx(tx, key)?;
        Ok(val.and_then(|v| v.parse().ok()).unwrap_or(default))
    }

    /// 主入口：执行催租 Agent
    pub fn run_agent<C: HasConnection>(
        &self,
        ctx: &C,
        req: &RunCollectionAgentRequest,
    ) -> Result<CollectionAgentReport> {
        info!(
            "[CollectionAgent] 启动扫描: dry_run={}, year_month={:?}, auto_mark={}",
            req.dry_run, req.year_month, req.auto_mark_violation
        );

        ctx.transaction(|tx| {
            // Step 1: 获取所有未支付账单
            let bills = queries::get_unpaid_bills_tx(tx, req.year_month.as_deref())?;

            // Step 2: 计算逾期天数并过滤
            let overdue_bills: Vec<OverdueBill> = bills
                .into_iter()
                .map(|mut b| {
                    b.overdue_days = Self::calculate_overdue_days(&b.due_date);
                    b.escalation_level = EscalationLevel::from_overdue_days(b.overdue_days);
                    b
                })
                .filter(|b| b.overdue_days > 0)
                .collect();

            info!(
                "[CollectionAgent] 扫描 {} 个账单，逾期 {} 个",
                overdue_bills.len(),
                overdue_bills.iter().filter(|b| b.overdue_days > 0).count()
            );

            // Step 3: 生成催租消息草稿
            let drafts: Vec<ReminderDraft> = overdue_bills
                .iter()
                .map(|b| self.generate_reminder_draft(b))
                .collect();

            // Step 4: 预览模式直接返回
            if req.dry_run {
                return Ok(self.build_report(&overdue_bills, &drafts, 0, 0, false));
            }

            // Step 5: 获取配置
            let min_reminder_days = self.get_config_i64(tx, "催租防打扰天数", 3)?;
            let auto_violation_enabled = self.get_config_bool(tx, "自动违约开关", false)?;
            let auto_violation_days = self.get_config_i64(tx, "自动违约天数阈值", 30)?;

            // Step 6: 创建催租提醒（幂等）
            let mut reminders_created = 0;
            for bill in &overdue_bills {
                if !self.should_create_reminder(tx, bill, min_reminder_days)? {
                    continue;
                }
                let draft = drafts.iter().find(|d| d.bill_id == bill.bill_id);
                if let Some(d) = draft {
                    queries::create_collection_reminder_tx(
                        tx,
                        bill.bill_id,
                        bill.room_id,
                        bill.lease_id,
                        &d.title,
                        &d.message,
                        bill.escalation_level.as_str(),
                    )?;
                    queries::update_bill_escalation_tx(tx, bill.bill_id, &bill.escalation_level)?;
                    reminders_created += 1;
                }
            }

            // Step 7: 可选违约标记
            let mut rooms_marked = 0;
            if req.auto_mark_violation && auto_violation_enabled {
                for bill in &overdue_bills {
                    if bill.overdue_days >= auto_violation_days
                        && bill.escalation_level == EscalationLevel::Severe
                    {
                        match queries::can_mark_violation_tx(tx, bill.room_id) {
                            Ok(true) => {
                                match queries::mark_room_violation_tx(tx, bill.room_id, bill.lease_id) {
                                    Ok(_) => {
                                        info!(
                                            "[CollectionAgent] 标记违约: room_id={}, overdue_days={}",
                                            bill.room_id, bill.overdue_days
                                        );
                                        rooms_marked += 1;
                                    }
                                    Err(e) => {
                                        info!(
                                            "[CollectionAgent] 标记违约失败: room_id={}, err={}",
                                            bill.room_id, e
                                        );
                                    }
                                }
                            }
                            Ok(false) => {}
                            Err(e) => {
                                info!(
                                    "[CollectionAgent] 检查违约条件失败: room_id={}, err={}",
                                    bill.room_id, e
                                );
                            }
                        }
                    }
                }
            }

            info!(
                "[CollectionAgent] 执行完成: reminders_created={}, rooms_marked={}",
                reminders_created, rooms_marked
            );

            Ok(self.build_report(&overdue_bills, &drafts, reminders_created, rooms_marked, true))
        })
    }
}
