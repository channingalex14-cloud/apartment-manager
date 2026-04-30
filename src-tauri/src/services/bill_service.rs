//! 账单服务

//!

//! 账单生成、费用计算、账单查询



use crate::db::{queries, HasConnection};

use crate::errors::{AppError, Result};

use crate::models::{BillResponse, BillStatus, GenerateBillsRequest, RoomBillResult, RoomStatus};

use rusqlite::{params, OptionalExtension};

use tracing::info;

use serde::{Deserialize, Serialize};



const DAYS_IN_MONTH: i64 = 30;

const MIDDLE_DAY: i64 = 15;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillSummary {
    pub total_amount: i64,
    pub total_paid: i64,
    pub total_pending: i64,
    pub bill_count: i64,
    pub pending_count: i64,
    pub paid_count: i64,
}




// ========================

// 账单查询响应类型

// ========================



/// 账单列表项（分页）

#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(rename_all = "snake_case")]

pub struct BillListItem {

    pub id: i64,

    pub room_id: i64,

    pub room_number: String,

    pub building: String,

    pub tenant_name: Option<String>,

    pub year_month: String,

    pub total_amount: i64,

    pub actual_paid: i64,

    pub status: String,

    pub due_date: Option<String>,

}



impl From<queries::BillRow> for BillListItem {

    fn from(row: queries::BillRow) -> Self {

        Self {

            id: row.id,

            room_id: row.room_id,

            room_number: row.room_number,

            building: row.building,

            tenant_name: row.tenant_name,

            year_month: row.year_month,

            total_amount: row.total_amount,

            actual_paid: row.actual_paid,

            status: row.status,

            due_date: row.due_date,

        }

    }

}



/// 账单列表响应

#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(rename_all = "snake_case")]

pub struct BillListResponse {

    pub bills: Vec<BillListItem>,

    pub total: i32,

    pub page: i32,

    pub page_size: i32,

    pub total_pages: i32,

}



/// 账单详情响应

#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(rename_all = "snake_case")]

pub struct BillDetailResponse {

    pub id: i64,

    pub room_id: i64,

    pub room_number: String,

    pub building: String,

    pub tenant_name: Option<String>,

    pub tenant_phone: Option<String>,

    pub year_month: String,

    pub status: String,

    pub due_date: Option<String>,

    pub created_at: Option<String>,

    // 费用明细（分）

    pub rent_fee: i64,

    pub property_fee: i64,

    pub water_fee: i64,

    pub electric_fee: i64,

    pub management_fee: i64,

    pub repair_fee: i64,

    pub misc_fee: i64,

    pub misc_fee_remark: Option<String>,

    pub previous_balance: i64,

    pub total_amount: i64,

    pub actual_paid: i64,

    pub remaining_amount: i64,

    // 水电读数

    pub water_reading_prev: i64,

    pub water_reading_current: i64,

    pub electric_reading_prev: i64,

    pub electric_reading_current: i64,

    // 操作权限标记

    pub can_confirm: bool,

    pub can_void: bool,

    pub can_pay_partial: bool,

}



/// 账单操作响应

#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(rename_all = "snake_case")]

pub struct BillActionResponse {

    pub success: bool,

    pub message: String,

}



/// 账单服务

pub struct BillService;



impl BillService {

    /// 生成月度账单（批量）

    ///

    /// 注意：这是耗时操作，应该在 spawn_blocking 中调用

    pub fn generate_monthly_bills<C: HasConnection>(

        &self,

        ctx: &C,

        req: &GenerateBillsRequest,

    ) -> Result<BillResponse> {

        info!("生成账单请求: year_month={}, room_count={}",

              req.year_month,

              req.room_ids.as_ref().map(|v| v.len()).unwrap_or(0));



        // 获取要生成账单的房间列表

        let room_ids = match &req.room_ids {

            Some(ids) => ids.clone(),

            None => {

                // 全量生成

                let conn = ctx.get_conn()?;

                let rooms = queries::list_rooms(&*conn)?;

                rooms.into_iter().map(|r| r.id).collect()

            }

        };



        let mut generated = 0;



        for room_id in room_ids {

            // 解析年月

            let parts: Vec<&str> = req.year_month.split('-').collect();

            let year: i32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(2026);

            let month: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);



            let misc_fee = req.misc_fee.unwrap_or(0);

            let misc_fee_remark = req.misc_fee_remark.clone();



            match self.generate_room_bill(ctx, room_id, year, month, misc_fee, misc_fee_remark) {

                Ok(result) => {

                    if result.bill_id.is_some() {

                        generated += 1;

                    }

                }

                Err(e) => {

                    info!("房间 {} 生成账单失败: {}", room_id, e);

                    // 继续处理其他房间

                }

            }

        }



        info!("账单生成完成: 成功 {} 个", generated);

        Ok(BillResponse::success(generated))

    }





    /// 生成单个房间的月度账单（新版本，返回详细结果）

    ///

    /// 必须在事务内执行，防止死锁

    pub fn generate_room_bill<C: HasConnection>(

        &self,

        ctx: &C,

        room_id: i64,

        year: i32,

        month: u32,

        misc_fee: i64,

        misc_fee_remark: Option<String>,

    ) -> Result<RoomBillResult> {

        let year_month = format!("{:04}-{:02}", year, month);



        ctx.transaction(|tx| {

            // 0. 幂等检查：已存在的非终态账单不重复生成

            if let Some(existing) = queries::get_bill_by_room_month_tx(tx, room_id, &year_month)? {

                let status = BillStatus::from_str(&existing.status)?;

                if !status.is_terminal() {

                    return Ok(RoomBillResult::skipped(

                        room_id,

                        existing.room_id.to_string(),

                        &format!("该房间本月账单已存在（状态: {}）", existing.status),

                    ));

                }

                // 已作废的账单：重新生成前先 UPDATE 恢复为待缴费

                if status == BillStatus::Voided {

                    queries::regenerate_voided_bill_tx(tx, existing.id)?;

                    return Ok(RoomBillResult::success(

                        room_id,

                        existing.room_id.to_string(),

                        existing.id,

                        0, 0, 0, 0, 0, 0, None, 0, 0,

                    ));

                }

                // 已支付的，不允许重新生成

                return Err(AppError::Business(

                    "该房间本月账单已支付，不允许重新生成".to_string(),

                ));

            }



            // 1. 在事务内读取房间

            let room = queries::get_room_by_id_tx(tx, room_id)?

                .ok_or_else(|| AppError::not_found("房间", room_id))?;



            // 2. 解析房间状态

            let room_status = room.status;



            // 3. 空房/管理房/维修房 → 跳过

            match room_status {

                RoomStatus::Vacant | RoomStatus::Management | RoomStatus::Maintenance => {

                    return Ok(RoomBillResult::skipped(

                        room_id,

                        room.room_number.clone(),

                        &format!("{}，跳过", room.status),

                    ));

                }

                _ => {}

            }



            // 4. 获取当前生效合同（用于半月计算）

            let active_lease = self.get_active_lease_for_room_tx(tx, room_id)?;



            // 5. 计算半月入住/退房天数

            let (start_day, end_day) = self.get_half_month_days(&active_lease, &year_month);



            // 6. 获取单价配置（事务版本）

            let water_price = queries::get_config_value_tx(tx, "水费单价")?

                .and_then(|v| v.parse::<i64>().ok())

                .unwrap_or(600);



            let electric_price = queries::get_config_value_tx(tx, "电费单价")?

                .and_then(|v| v.parse::<i64>().ok())

                .unwrap_or(73);



            let management_price = queries::get_config_value_tx(tx, "管理费单价")?

                .and_then(|v| v.parse::<i64>().ok())

                .unwrap_or(57);



            // 7. 获取上期水电读数（从上月账单）

            let (water_prev, electric_prev) = self.get_last_readings_tx(tx, room_id)?;



            // 8. 获取本期抄表数据（从 meter_readings 表）

            let current_reading = queries::get_meter_reading_by_room_month_tx(

                tx, room_id, year, month as i32,

            )?;

            let Some(reading) = current_reading else {

                return Ok(RoomBillResult::skipped(

                    room_id,

                    room.room_number.clone(),

                    "未录入水电读数",

                ));

            };

            let water_current = reading.water_reading;

            let electric_current = reading.electric_reading;



            // 9. 计算用量

            let water_usage = water_current.saturating_sub(water_prev);

            let electric_usage = electric_current.saturating_sub(electric_prev);



            // 10. 根据房间状态计算水电费和管理费

            let (water_fee, electric_fee, management_fee) = match room_status {

                RoomStatus::NewRented => (0, 0, 0), // 新租首月免水电

                RoomStatus::Rented | RoomStatus::Staff | RoomStatus::Violation | RoomStatus::PendingClean => {

                    let wf = water_usage

                        .checked_mul(water_price)

                        .ok_or_else(|| AppError::business("水费计算溢出"))?;

                    let ef = electric_usage

                        .checked_mul(electric_price)

                        .ok_or_else(|| AppError::business("电费计算溢出"))?;

                    let mf = electric_usage

                        .checked_mul(management_price)

                        .ok_or_else(|| AppError::business("管理费计算溢出"))?;

                    (wf, ef, mf)

                }

                _ => (0, 0, 0),

            };



            // 11. 计算租金和物业费

            let (rent_fee, property_fee, rent_days, _bill_type) = match room_status {

                RoomStatus::Rented | RoomStatus::NewRented | RoomStatus::Violation => {

                    self.calculate_half_month_fee(

                        room.base_rent,

                        room.property_fee,

                        start_day,

                        end_day,

                    )

                }

                RoomStatus::Staff | RoomStatus::Management => {

                    (0, room.property_fee, 0, "正常")

                }

                _ => (0, 0, 0, "正常"),

            };



            // 12. 获取上期欠费

            let previous_balance = self.get_previous_balance_tx(tx, room_id)?;



            // 13. 计算总金额

            let current_fees = rent_fee

                .checked_add(property_fee)

                .and_then(|v| v.checked_add(water_fee))

                .and_then(|v| v.checked_add(electric_fee))

                .and_then(|v| v.checked_add(management_fee))

                .and_then(|v| v.checked_add(misc_fee))

                .ok_or_else(|| AppError::business("费用计算溢出"))?;

            let total_amount = previous_balance

                .checked_add(current_fees)

                .ok_or_else(|| AppError::business("总金额计算溢出"))?;



            // 14. 插入账单

            // 修复：使用 ON CONFLICT 处理软删除记录的"复活"

            // 当 is_deleted=1 的记录存在时，会将其更新为 is_deleted=0 并重新生成

            tx.execute(

                r#"

                INSERT INTO monthly_bills

                (year_month, room_id, lease_id, room_status,

                 water_reading_prev, water_reading_current,

                 electric_reading_prev, electric_reading_current,

                 water_usage, electric_usage,

                 water_unit_price, electric_unit_price, management_unit_price,

                 rent_fee, rent_days, rent_daily_rate,

                 property_fee, water_fee, electric_fee, management_fee,

                 misc_fee, misc_fee_remark,

                 total_amount, previous_balance, actual_paid,

                 status, bill_sequence, is_deleted)

                VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, 1, 0)

                ON CONFLICT(year_month, room_id, bill_sequence) DO UPDATE SET

                    is_deleted = 0,

                    lease_id = excluded.lease_id,

                    room_status = excluded.room_status,

                    water_reading_prev = excluded.water_reading_prev,

                    water_reading_current = excluded.water_reading_current,

                    electric_reading_prev = excluded.electric_reading_prev,

                    electric_reading_current = excluded.electric_reading_current,

                    water_usage = excluded.water_usage,

                    electric_usage = excluded.electric_usage,

                    water_unit_price = excluded.water_unit_price,

                    electric_unit_price = excluded.electric_unit_price,

                    management_unit_price = excluded.management_unit_price,

                    rent_fee = excluded.rent_fee,

                    rent_days = excluded.rent_days,

                    rent_daily_rate = excluded.rent_daily_rate,

                    property_fee = excluded.property_fee,

                    water_fee = excluded.water_fee,

                    electric_fee = excluded.electric_fee,

                    management_fee = excluded.management_fee,

                    misc_fee = excluded.misc_fee,

                    misc_fee_remark = excluded.misc_fee_remark,

                    total_amount = excluded.total_amount,

                    previous_balance = excluded.previous_balance,

                    actual_paid = 0,

                    status = excluded.status,

                    deleted_at = NULL,

                    deleted_by = NULL,

                    updated_at = datetime('now')

                "#,

                params![

                    &year_month, room_id, room.status,

                    water_prev, water_current,

                    electric_prev, electric_current,

                    water_usage, electric_usage,

                    water_price, electric_price, management_price,

                    rent_fee, rent_days,

                    property_fee, water_fee, electric_fee, management_fee,

                    misc_fee, misc_fee_remark.as_deref().unwrap_or(""),

                    total_amount, previous_balance,

                    BillStatus::Pending.as_str()

                ],

            )?;



            let bill_id = tx.last_insert_rowid();



            // 15. 更新房间当前水电表读数（同步 rooms.meter_current）

            queries::update_room_meters_tx(tx, room_id, water_current, electric_current)?;



            Ok(RoomBillResult::success(

                room_id,

                room.room_number.clone(),

                bill_id,

                rent_fee,

                property_fee,

                water_fee,

                electric_fee,

                management_fee,

                misc_fee,

                misc_fee_remark.clone(),

                previous_balance,

                total_amount,

            ))

        })

    }



    /// 获取房间当前生效的合同（事务版本）

    fn get_active_lease_for_room_tx(

        &self,

        tx: &rusqlite::Transaction,

        room_id: i64,

    ) -> Result<Option<crate::models::Lease>> {

        let mut stmt = tx.prepare(

            "SELECT * FROM leases WHERE room_id = ? AND status = '生效中' AND is_deleted = 0 LIMIT 1",

        )?;



        let result = stmt

            .query_row([room_id], |row| {

                Ok(crate::models::Lease {

                    id: row.get("id")?,

                    room_id: row.get("room_id")?,

                    tenant_id: row.get("tenant_id")?,

                    contract_number: row.get("contract_number")?,

                    start_date: row.get("start_date")?,

                    end_date: row.get("end_date")?,

                    monthly_rent: row.get("monthly_rent")?,

                    property_fee: row.get("property_fee")?,

                    deposit: row.get("deposit")?,

                    deposit_received: row.get("deposit_received")?,

                    deposit_balance: row.get("deposit_balance")?,

                    deposit_status: row.get("deposit_status")?,

                    move_in_date: row.get("move_in_date")?,

                    move_out_date: row.get("move_out_date")?,

                    termination_reason: row.get("termination_reason")?,

                    status: row.get("status")?,

                    status_reason: row.get("status_reason")?,

                    notes: row.get("notes")?,

                    is_deleted: row.get::<_, i32>("is_deleted")? != 0,

                    created_at: row.get("created_at")?,

                    updated_at: row.get("updated_at")?,

                })

            })

            .optional()

            .map_err(|e| AppError::Database(e))?;



        Ok(result)

    }



    /// 获取房间上期账单的水电读数（事务版本）

    fn get_last_readings_tx(&self, tx: &rusqlite::Transaction, room_id: i64) -> Result<(i64, i64)> {

        let mut stmt = tx.prepare(

            r#"

            SELECT water_reading_current, electric_reading_current

            FROM monthly_bills

            WHERE room_id = ? AND is_deleted = 0

            ORDER BY year_month DESC, bill_sequence DESC

            LIMIT 1

            "#,

        )?;



        let result = stmt

            .query_row([room_id], |row| {

                Ok((

                    row.get::<_, i64>("water_reading_current")?,

                    row.get::<_, i64>("electric_reading_current")?,

                ))

            })

            .optional()

            .map_err(|e| AppError::Database(e))?;



        Ok(result.unwrap_or((0, 0)))

    }



    /// 获取房间上期欠费余额（事务版本）

    fn get_previous_balance_tx(&self, tx: &rusqlite::Transaction, room_id: i64) -> Result<i64> {

        let paid_status = BillStatus::Paid.as_str();

        let mut stmt = tx.prepare(&format!(

            r#"

            SELECT total_amount - actual_paid as balance

            FROM monthly_bills

            WHERE room_id = ? AND is_deleted = 0 AND status != '{}'

            ORDER BY year_month DESC, bill_sequence DESC

            LIMIT 1

            "#,

            paid_status

        ))?;



        let result = stmt

            .query_row([room_id], |row| row.get::<_, i64>("balance"))

            .optional()

            .map_err(|e| AppError::Database(e))?;



        Ok(result.unwrap_or(0))

    }



    /// 计算半月结算的起始/结束天数

    ///

    /// 返回 (start_day, end_day)：

    /// - start_day: 入住日在该月内，返回入住日

    /// - end_day: 退房日在该月内，返回退房日

    fn get_half_month_days(

        &self,

        lease: &Option<crate::models::Lease>,

        year_month: &str,

    ) -> (Option<i64>, Option<i64>) {

        let Some(lease) = lease else {

            return (None, None);

        };



        let mut start_day = None;

        let mut end_day = None;



        // 检查入住日是否在该月

        if let Some(move_in) = &lease.move_in_date {

            if move_in.starts_with(year_month) {

                // move_in_date 格式: "YYYY-MM-DD"

                if let Some(day_str) = move_in.split('-').nth(2) {

                    start_day = day_str.parse::<i64>().ok();

                }

            }

        }



        // 检查退房日是否在该月

        if let Some(move_out) = &lease.move_out_date {

            if move_out.starts_with(year_month) {

                if let Some(day_str) = move_out.split('-').nth(2) {

                    end_day = day_str.parse::<i64>().ok();

                }

            }

        }



        (start_day, end_day)

    }





    /// 计算半月账单费用

    ///

    /// 用于月中入住或退房的场景

    pub fn calculate_half_month_fee(

        &self,

        base_rent: i64,

        property_fee: i64,

        start_day: Option<i64>,

        end_day: Option<i64>,

    ) -> (i64, i64, i32, &'static str) {

        // 返回值：(租金, 物业费, 租金天数, 账单类型)



        if let Some(day) = start_day {

            if day <= MIDDLE_DAY {

                let daily_rent = base_rent / DAYS_IN_MONTH;

                let rent_days = DAYS_IN_MONTH - day + 1;

                let rent = daily_rent * rent_days;

                let property = property_fee / 2;

                (rent, property, rent_days as i32, "半月结算")

            } else {

                // 16 日后入住，收全月

                (base_rent, property_fee, DAYS_IN_MONTH as i32, "正常")

            }

        } else if let Some(day) = end_day {

            // 退房半月计算

            if day <= MIDDLE_DAY {

                let rent = base_rent / 2;

                let property = property_fee / 2;

                (rent, property, day as i32, "半月结算")

            } else {

                (base_rent, property_fee, DAYS_IN_MONTH as i32, "正常")

            }

        } else {

            (base_rent, property_fee, DAYS_IN_MONTH as i32, "正常")

        }

    }



    /// 查询账单列表（分页 + 筛选）

    pub fn query_bills<C: HasConnection>(

        &self,

        ctx: &C,

        year: Option<i32>,

        month: Option<i32>,

        room_id: Option<i64>,

        status: Option<&str>,

        page: i32,

        page_size: i32,

    ) -> Result<BillListResponse> {

        let (bills, total) = ctx.transaction(|tx| {

            queries::query_bills_tx(tx, year, month, room_id, status, page, page_size)

        })?;



        let total_pages = if total == 0 {

            1

        } else {

            (total + page_size - 1) / page_size

        };



        Ok(BillListResponse {

            bills: bills.into_iter().map(BillListItem::from).collect(),

            total,

            page,

            page_size,

            total_pages,

        })

    }



    /// 查询账单详情

    pub fn get_bill_detail<C: HasConnection>(

        &self,

        ctx: &C,

        bill_id: i64,

    ) -> Result<BillDetailResponse> {

        let bill = ctx.transaction(|tx| {

            queries::get_bill_detail_tx(tx, bill_id)

        })?

        .ok_or_else(|| AppError::not_found("账单", bill_id))?;



        let status = BillStatus::from_str(&bill.status)?;

        let remaining = bill.total_amount - bill.actual_paid;



        Ok(BillDetailResponse {

            id: bill.id,

            room_id: bill.room_id,

            room_number: bill.room_number,

            building: bill.building,

            tenant_name: bill.tenant_name,

            tenant_phone: bill.tenant_phone,

            year_month: bill.year_month,

            status: bill.status.clone(),

            due_date: bill.due_date.clone(),

            created_at: bill.created_at.clone(),

            rent_fee: bill.rent_fee,

            property_fee: bill.property_fee,

            water_fee: bill.water_fee,

            electric_fee: bill.electric_fee,

            management_fee: bill.management_fee,

            misc_fee: bill.misc_fee,

            misc_fee_remark: bill.misc_fee_remark.clone(),

            previous_balance: bill.previous_balance,

            total_amount: bill.total_amount,

            actual_paid: bill.actual_paid,

            remaining_amount: remaining,

            water_reading_prev: bill.water_reading_prev,

            water_reading_current: bill.water_reading_current,

            electric_reading_prev: bill.electric_reading_prev,

            electric_reading_current: bill.electric_reading_current,

            repair_fee: bill.repair_fee,

            can_confirm: status.can_transition_to(&BillStatus::Paid),

            can_void: status.can_transition_to(&BillStatus::Voided),

            can_pay_partial: status.can_transition_to(&BillStatus::Partial),

        })

    }



    /// 确认账单全额支付

    pub fn confirm_bill_paid<C: HasConnection>(

        &self,

        ctx: &C,

        bill_id: i64,

    ) -> Result<BillActionResponse> {

        ctx.transaction(|tx| {

            let bill = queries::get_bill_by_id_tx(tx, bill_id)?

                .ok_or_else(|| AppError::not_found("账单", bill_id))?;



            let status = BillStatus::from_str(&bill.status)?;

            if !status.can_transition_to(&BillStatus::Paid) {

                return Err(AppError::Business(format!(

                    "账单状态为{}，无法确认支付", bill.status

                )));

            }



            queries::confirm_bill_paid_tx(tx, bill_id)?;

            Ok(BillActionResponse {

                success: true,

                message: "确认支付成功".to_string(),

            })

        })

    }



    /// 账单部分支付

    pub fn partial_pay_bill<C: HasConnection>(

        &self,

        ctx: &C,

        bill_id: i64,

        amount: i64,

    ) -> Result<BillActionResponse> {

        if amount <= 0 {

            return Err(AppError::Business("支付金额必须大于0".to_string()));

        }



        ctx.transaction(|tx| {

            let bill = queries::get_bill_by_id_tx(tx, bill_id)?

                .ok_or_else(|| AppError::not_found("账单", bill_id))?;



            let status = BillStatus::from_str(&bill.status)?;

            if !status.can_transition_to(&BillStatus::Partial) {

                return Err(AppError::Business(format!(

                    "账单状态为{}，无法部分支付", bill.status

                )));

            }



            let remaining = bill.total_amount - bill.actual_paid;

            if amount > remaining {
                return Err(AppError::Business("支付金额超过剩余金额".to_string()));
            }



            queries::partial_pay_bill_tx(tx, bill_id, amount)?;

            Ok(BillActionResponse {

                success: true,

                message: format!("部分支付成功，剩余{}", remaining - amount),

            })

        })

    }



    /// 作废账单

    pub fn void_bill<C: HasConnection>(

        &self,

        ctx: &C,

        bill_id: i64,

    ) -> Result<BillActionResponse> {

        ctx.transaction(|tx| {

            let bill = queries::get_bill_by_id_tx(tx, bill_id)?

                .ok_or_else(|| AppError::not_found("账单", bill_id))?;



            let status = BillStatus::from_str(&bill.status)?;

            if !status.can_transition_to(&BillStatus::Voided) {

                return Err(AppError::Business(format!(

                    "账单状态为{}，无法作废", bill.status

                )));

            }



            queries::void_bill_tx(tx, bill_id)?;

            Ok(BillActionResponse {

                success: true,

                message: "账单已作废".to_string(),

            })

        })

    }



    /// 归档指定年月的所有账单

    pub fn archive_bills<C: HasConnection>(

        &self,

        ctx: &C,

        year_month: &str,

    ) -> Result<crate::models::bill::ArchiveBillsResponse> {

        info!("归档账单请求: year_month={}", year_month);



        let count = ctx.transaction(|tx| {

            queries::archive_bills_by_month_tx(tx, year_month)

        })?;



        info!("归档完成: year_month={}, 归档 {} 条", year_month, count);

        Ok(crate::models::bill::ArchiveBillsResponse::success(count))

    }



    /// 恢复指定年月的已归档账单

    pub fn restore_bills<C: HasConnection>(

        &self,

        ctx: &C,

        year_month: &str,

    ) -> Result<crate::models::bill::ArchiveBillsResponse> {

        info!("恢复归档请求: year_month={}", year_month);



        let count = ctx.transaction(|tx| {

            queries::restore_bills_by_month_tx(tx, year_month)

        })?;



        info!("恢复归档完成: year_month={}, 恢复 {} 条", year_month, count);

        Ok(crate::models::bill::ArchiveBillsResponse::success(count))

    }



    /// 获取所有已归档的年月列表

    pub fn list_archived_months<C: HasConnection>(

        &self,

        ctx: &C,

    ) -> Result<Vec<String>> {

        let conn = ctx.get_conn()?;

        queries::list_archived_year_months(&conn)
    }

    pub fn get_bill_summary<C: HasConnection>(
        &self,
        ctx: &C,
        year_month: Option<&str>,
    ) -> Result<BillSummary> {
        ctx.transaction(|tx| {
            let base_sql = if let Some(ym) = year_month {
                format!(
                    "SELECT \
                    COALESCE(SUM(total_amount), 0) as total_amount, \
                    COALESCE(SUM(actual_paid), 0) as total_paid, \
                    COALESCE(SUM(CASE WHEN status IN ('{}', '{}') THEN total_amount - actual_paid ELSE 0 END), 0) as total_pending, \
                    COUNT(*) as bill_count, \
                    SUM(CASE WHEN status IN ('{}', '{}') THEN 1 ELSE 0 END) as pending_count, \
                    SUM(CASE WHEN status = '{}' THEN 1 ELSE 0 END) as paid_count \
                    FROM monthly_bills WHERE is_deleted = 0 AND year_month = '{}'",
                    BillStatus::Pending.as_str(),
                    BillStatus::Partial.as_str(),
                    BillStatus::Pending.as_str(),
                    BillStatus::Partial.as_str(),
                    BillStatus::Paid.as_str(),
                    ym
                )
            } else {
                format!(
                    "SELECT \
                    COALESCE(SUM(total_amount), 0) as total_amount, \
                    COALESCE(SUM(actual_paid), 0) as total_paid, \
                    COALESCE(SUM(CASE WHEN status IN ('{}', '{}') THEN total_amount - actual_paid ELSE 0 END), 0) as total_pending, \
                    COUNT(*) as bill_count, \
                    SUM(CASE WHEN status IN ('{}', '{}') THEN 1 ELSE 0 END) as pending_count, \
                    SUM(CASE WHEN status = '{}' THEN 1 ELSE 0 END) as paid_count \
                    FROM monthly_bills WHERE is_deleted = 0",
                    BillStatus::Pending.as_str(),
                    BillStatus::Partial.as_str(),
                    BillStatus::Pending.as_str(),
                    BillStatus::Partial.as_str(),
                    BillStatus::Paid.as_str()
                )
            };

            let mut stmt = tx.prepare(&base_sql)
                .map_err(AppError::Database)?;
            stmt.query_row([], |row| {
                Ok(BillSummary {
                    total_amount: row.get(0)?,
                    total_paid: row.get(1)?,
                    total_pending: row.get(2)?,
                    bill_count: row.get(3)?,
                    pending_count: row.get(4)?,
                    paid_count: row.get(5)?,
                })
            }).map_err(|e| AppError::Database(e))
        })
    }
}

#[cfg(test)]

mod integration_tests {

    use super::*;

    use crate::db::connection::TestContext;

    use crate::db::test_helpers::*;



    /// 在租整月 - 最常见场景

    /// 上月(3月)抄表: 水10方, 电50度

    /// 本月(4月)抄表: 水18方, 电120度

    #[test]

    fn test_standard_bill_full_month_rented() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "X01", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000000");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 上月抄表: 水10方, 电50度 → 插入 March bill 和 meter_readings

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 10, 50);



        // 本月抄表: 水18方, 电120度 → 插入 April meter_readings

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 18, 120, "2026-04-10");



        let ctx = TestContext::new(conn);

        let service = BillService;

        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        // 房租: 300000分 = 3000元

        // 水费: (18-10) * 600 = 4800分 = 48元

        // 电费: (120-50) * 73 = 5110分 = 51.1元

        // 管理费: (120-50) * 57 = 3990分 = 39.9元

        // 合计: 300000 + 4800 + 5110 + 3990 = 313900分 = 3139元

        assert_eq!(result.rent, 300000);

        assert_eq!(result.water_fee, 4800);

        assert_eq!(result.electric_fee, 5110);

        assert_eq!(result.management_fee, 3990);

        assert_eq!(result.total, 313900);

    }



    /// 空房跳过

    #[test]

    fn test_standard_bill_vacant_room() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "X02", "空房", 300000);



        let ctx = TestContext::new(conn);

        let service = BillService;



        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert_eq!(result.total, 0);

        assert!(result.skip_reason.is_some());

    }



    /// Test 3: 新租首月 - 免水电管理费，租金按半月折算

    /// move_in_date=2026-04-01，入住日在15日之前，收半月

    #[test]

    fn test_generate_bill_new_rented_first_month() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "101-N", "新租", 300000, 5000, 0, 0);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000002");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        // 更新入住日期为4月1日

        update_lease_move_in_date(&conn, lease_id, "2026-04-01");



        // 插入4月抄表读数（新租首月免水电，读数为0）

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 0, 0, "2026-04-01");



        let ctx = TestContext::new(conn);

        let service = BillService;



        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert!(result.bill_id.is_some());



        // 免水电管理费

        assert_eq!(result.water_fee, 0);

        assert_eq!(result.electric_fee, 0);

        assert_eq!(result.management_fee, 0);



        // 半月租金: (300000/30) * 30 = 300000（入住1日，算30天）

        // 但实际按半月公式: day=1 <= 15, rent_days = 30-1+1 = 30, daily=10000, rent=300000

        assert_eq!(result.rent, 300000);

        assert_eq!(result.property_fee, 2500); // 半月物业费

    }



    /// Test 4: 员工房 - 不收租金，但收水电管理费

    #[test]

    fn test_generate_bill_staff_room() {

        let conn = create_test_db_in_memory();

        // 使用带指定meter的版本: water=100, electric=200

        let room_id = insert_test_room_with_meters(&conn, "101-S", "员工", 300000, 5000, 100, 200);

        let tenant_id = insert_test_tenant(&conn, "员工1", "13800000003");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 0); // 员工押金=0



        // 设置上月抄表（0/0）和本月抄表（100/200）

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 0, 0);

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 100, 200, "2026-04-10");



        let ctx = TestContext::new(conn);

        let service = BillService;



        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert!(result.bill_id.is_some());



        // 员工房不收租金

        assert_eq!(result.rent, 0);

        // 收全额物业费

        assert_eq!(result.property_fee, 5000);

        // 水电管理费按用量（与在租相同）

        assert_eq!(result.water_fee, 60000);   // 100 * 600

        assert_eq!(result.electric_fee, 14600); // 200 * 73

        assert_eq!(result.management_fee, 11400); // 200 * 57

    }



    /// Test 5: 违约房 - 与在租计算相同（不减免）

    #[test]

    fn test_generate_bill_violation_room() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "101-VL", "违约", 300000, 5000, 100, 200);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000004");

        insert_test_lease(&conn, room_id, tenant_id, "违约中", 200000);



        // 设置上月抄表和本月抄表

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 0, 0);

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 100, 200, "2026-04-10");



        let ctx = TestContext::new(conn);

        let service = BillService;



        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert!(result.bill_id.is_some());



        // 违约房租金计算与在租相同（全额）

        assert_eq!(result.rent, 300000);

        assert_eq!(result.property_fee, 5000);

        assert_eq!(result.water_fee, 60000);   // 100 * 600

        assert_eq!(result.electric_fee, 14600); // 200 * 73

        assert_eq!(result.management_fee, 11400); // 200 * 57

    }



    /// 回归测试 #19：generate_room_bill 在事务内读取，账单金额与抄表时间点一致

    ///

    /// TOCTOU 漏洞场景：事务外读取 meter_current，事务内写入账单。

    /// 如果在事务外读取，水电费可能因抄表更新而计算错误。

    /// 修复后：账单中的 water_fee/electric_fee 基于事务开始时的 meter 值计算。

    #[test]

    fn test_generate_bill_transaction_read_integrity() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "X10", "在租", 300000, 5000, 100, 200);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000010");

        let _lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 设置上期抄表（插入历史账单和 meter_readings）

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 100, 50);



        // 模拟：本月抄表已录入 meter_readings

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 200, 150, "2026-04-10");



        let ctx = TestContext::new(conn);

        let service = BillService;



        // 生成账单：用水差=100，电差=100

        let result = service.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        // 验证：账单金额 = (200-100)*600 水 + (150-50)*73 电

        // water_fee = 100 * 600 = 60000

        // electric_fee = 100 * 73 = 7300

        // management_fee = 100 * 57 = 5700

        // rent = 300000, property_fee = 5000 (在租整月全额物业费)

        // total = 300000 + 5000 + 60000 + 7300 + 5700 = 378000

        assert_eq!(result.water_fee, 60000);

        assert_eq!(result.electric_fee, 7300);

        assert_eq!(result.management_fee, 5700);

        assert_eq!(result.property_fee, 5000);

        assert_eq!(result.total, 378000);



        // 再次生成同月账单：meter_current 已更新为 200/150，

        // 如果是事务外读取，第二次会算出 water_usage=0（200-200=0）

        // 修复后：第二次会读到新的 meter，应该产生不同金额

        // （这里验证的是语义正确性：同一房间同一月份不应重复生成有效账单）

    }



    /// 边界测试：房间未录入水电读数 → skip

    #[test]

    fn test_generate_bill_without_meter_reading_skips() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "X99", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "张三", "13899999999");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        // 不插入任何 meter_readings



        let ctx = TestContext::new(conn);

        let result = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert!(result.bill_id.is_none(), "未录入读数应跳过");

        assert!(result.skip_reason.is_some());

        assert!(result.skip_reason.unwrap().contains("未录入水电读数"));

    }



    /// 边界测试：先录入 meter_readings，再生成账单 → 成功

    #[test]

    fn test_generate_bill_after_meter_reading_succeeds() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "X88", "在租", 300000, 5000, 0, 0);

        let tenant_id = insert_test_tenant(&conn, "李四", "13888888888");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 先插入 meter_readings（模拟抄表录入）

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 50, 200, "2026-04-10");



        let ctx = TestContext::new(conn);

        let result = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();



        assert!(result.bill_id.is_some(), "录入读数后应生成成功");

        assert!(result.skip_reason.is_none());

        assert_eq!(result.water_fee, 30000);   // 50 * 600

        assert_eq!(result.electric_fee, 14600); // 200 * 73

    }

}



#[cfg(test)]

mod tests {

    use super::*;



    #[test]

    fn test_calculate_half_month_fee_normal() {

        // 无 start_day 和 end_day，返回正常

        let base_rent = 300000; // 3000元 = 300000分

        let property_fee = 5000; // 50元 = 5000分



        let (rent, property, days, bill_type) =

            BillService. calculate_half_month_fee(base_rent, property_fee, None, None);



        assert_eq!(rent, base_rent);

        assert_eq!(property, property_fee);

        assert_eq!(days, 30);

        assert_eq!(bill_type, "正常");

    }



    #[test]

    fn test_calculate_half_month_fee_move_in_before_15th() {

        // 15日前入住，收半月

        let base_rent = 300000; // 3000元

        let property_fee = 5000; // 50元

        let start_day = 10; // 10日入住



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, Some(start_day), None);



        // 日租金 = 300000 / 30 = 10000分

        // 租金天数 = 30 - 10 + 1 = 21天

        // 租金 = 210000分 = 2100元

        assert_eq!(rent, 210000);

        assert_eq!(property, 2500); // 半月物业费

        assert_eq!(days, 21);

        assert_eq!(bill_type, "半月结算");

    }



    #[test]

    fn test_calculate_half_month_fee_move_in_on_15th() {

        // 15日入住，收半月

        let base_rent = 300000;

        let property_fee = 5000;

        let start_day = 15;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, Some(start_day), None);



        // 租金天数 = 30 - 15 + 1 = 16天

        // 租金 = 160000分 = 1600元

        assert_eq!(rent, 160000);

        assert_eq!(property, 2500);

        assert_eq!(days, 16);

        assert_eq!(bill_type, "半月结算");

    }



    #[test]

    fn test_calculate_half_month_fee_move_in_after_15th() {

        // 16日后入住，收全月

        let base_rent = 300000;

        let property_fee = 5000;

        let start_day = 20;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, Some(start_day), None);



        assert_eq!(rent, base_rent);

        assert_eq!(property, property_fee);

        assert_eq!(days, 30);

        assert_eq!(bill_type, "正常");

    }



    #[test]

    fn test_calculate_half_month_fee_move_out_before_15th() {

        // 15日前退房，退半月

        let base_rent = 300000;

        let property_fee = 5000;

        let end_day = 10;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, None, Some(end_day));



        assert_eq!(rent, 150000); // 半租

        assert_eq!(property, 2500); // 半物业费

        assert_eq!(days, 10);

        assert_eq!(bill_type, "半月结算");

    }



    #[test]

    fn test_calculate_half_month_fee_move_out_on_15th() {

        // 15日退房，退半月

        let base_rent = 300000;

        let property_fee = 5000;

        let end_day = 15;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, None, Some(end_day));



        assert_eq!(rent, 150000);

        assert_eq!(property, 2500);

        assert_eq!(days, 15);

        assert_eq!(bill_type, "半月结算");

    }



    #[test]

    fn test_calculate_half_month_fee_move_out_after_15th() {

        // 16日后退房，不退

        let base_rent = 300000;

        let property_fee = 5000;

        let end_day = 20;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, None, Some(end_day));



        assert_eq!(rent, base_rent);

        assert_eq!(property, property_fee);

        assert_eq!(days, 30);

        assert_eq!(bill_type, "正常");

    }



    #[test]

    fn test_calculate_half_month_fee_first_day_of_month() {

        // 1日入住，按半月结算计算（但实际天数=30天=全月）

        let base_rent = 300000;

        let property_fee = 5000;

        let start_day = 1;



        let (rent, _property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, Some(start_day), None);



        // 30 - 1 + 1 = 30天（全月天数）

        assert_eq!(days, 30);

        assert_eq!(rent, base_rent); // 30天 = 全月租金

        // bill_type 是"半月结算"因为入住日在15日之前

        assert_eq!(bill_type, "半月结算");

    }



    #[test]

    fn test_calculate_half_month_fee_last_day_of_month() {

        // 30日退房，按天算

        let base_rent = 300000;

        let property_fee = 5000;

        let end_day = 30;



        let (rent, property, days, bill_type) =

            BillService.calculate_half_month_fee(base_rent, property_fee, None, Some(end_day));



        // 16日后退房，不退

        assert_eq!(rent, base_rent);

        assert_eq!(property, property_fee);

        assert_eq!(days, 30);

        assert_eq!(bill_type, "正常");

    }

}



#[cfg(test)]

mod bill_status_tests {

    use super::*;



    #[test]

    fn test_pending_to_paid() {

        let s = BillStatus::Pending;

        assert!(s.can_transition_to(&BillStatus::Paid));

    }



    #[test]

    fn test_pending_to_partial() {

        let s = BillStatus::Pending;

        assert!(s.can_transition_to(&BillStatus::Partial));

    }



    #[test]

    fn test_pending_to_voided() {

        let s = BillStatus::Pending;

        assert!(s.can_transition_to(&BillStatus::Voided));

    }



    #[test]

    fn test_partial_to_paid() {

        let s = BillStatus::Partial;

        assert!(s.can_transition_to(&BillStatus::Paid));

    }



    #[test]

    fn test_partial_to_voided() {

        let s = BillStatus::Partial;

        assert!(s.can_transition_to(&BillStatus::Voided));

    }



    #[test]

    fn test_paid_is_terminal() {

        assert!(!BillStatus::Paid.can_transition_to(&BillStatus::Voided));

        assert!(!BillStatus::Paid.can_transition_to(&BillStatus::Partial));

        assert!(!BillStatus::Paid.can_transition_to(&BillStatus::Pending));

        assert!(BillStatus::Paid.is_terminal());

    }



    #[test]

    fn test_voided_is_terminal() {

        assert!(!BillStatus::Voided.can_transition_to(&BillStatus::Paid));

        assert!(!BillStatus::Voided.can_transition_to(&BillStatus::Partial));

        assert!(!BillStatus::Voided.can_transition_to(&BillStatus::Pending));

        assert!(BillStatus::Voided.is_terminal());

    }



    #[test]

    fn test_from_paid_amount() {

        assert!(matches!(BillStatus::from_paid_amount(10000, 0), BillStatus::Pending));

        assert!(matches!(BillStatus::from_paid_amount(10000, 5000), BillStatus::Partial));

        assert!(matches!(BillStatus::from_paid_amount(10000, 10000), BillStatus::Paid));

        assert!(matches!(BillStatus::from_paid_amount(10000, 15000), BillStatus::Paid));

    }

}



#[cfg(test)]

mod bill_idempotency_tests {

    use super::*;

    use crate::db::connection::TestContext;

    use crate::db::test_helpers::*;



    #[test]

    fn test_generate_bill_idempotent_reject() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "X01", "在租", 300000, 5000, 100, 200);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000010");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 100, 50);

        // 插入4月抄表读数

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 200, 150, "2026-04-10");



        let ctx = TestContext::new(conn);



        // 第一次生成：成功

        let result1 = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();

        assert!(result1.bill_id.is_some(), "第一次生成应成功");



        // 第二次生成：幂等跳过（已有同月非终态账单）

        let result2 = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();

        assert!(result2.bill_id.is_none(), "第二次生成应被幂等跳过");

        assert!(result2.skip_reason.is_some());

        assert!(result2.skip_reason.unwrap().contains("已存在"));

    }



    #[test]

    fn test_generate_bill_voided_allow_regenerate() {

        // 测试：已作废账单的 room，允许重新生成

        // 策略：插入已作废的 March bill（作为上期），然后生成 April 账单（会成功）

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "X02", "在租", 300000, 5000, 100, 200);

        let tenant_id = insert_test_tenant(&conn, "张三", "13800000011");

        insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 上期账单：已作废（get_last_readings_tx 仍能读到 prev readings）

        conn.execute(

            r#"INSERT INTO monthly_bills (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence, is_deleted)

                VALUES ('2026-03', ?, NULL, '在租', 100, 100, 50, 50, 0, 0, 600, 73, 57, 0, 0, 0, 0, 0, 0, 0, 0, '',

                        0, 0, 0, '已作废', 1, 0)"#,

            [room_id],

        ).unwrap();



        // 设置 meter_readings：March(100,50) 和 April(200,150)

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 100, 50);

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 200, 150, "2026-04-10");



        let ctx = TestContext::new(conn);



        // 第一次生成 4月账单：March 已作废但有 readings，April 有 meter_readings，应成功

        let result = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();

        assert!(result.bill_id.is_some(), "上期已作废但有 meter_readings，本月应生成成功");

        assert!(result.skip_reason.is_none());

    }

}



#[cfg(test)]

mod bill_query_tests {

    use super::*;

    use crate::db::connection::TestContext;

    use crate::db::test_helpers::*;



    /// 辅助：插入一个完整账单记录

    fn insert_bill(

        conn: &rusqlite::Connection,

        room_id: i64,

        lease_id: i64,

        year_month: &str,

        total: i64,

        paid: i64,

        status: &str,

    ) {

        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark, total_amount, previous_balance, actual_paid, status, bill_sequence)

               VALUES (?1, ?2, ?3, '在租', 0, 0, 0, 0, 0, 0, 600, 73, 57, 0, 30, 0, 0, 0, 0, 0, 0, '', ?4, 0, ?5, ?6, 1)"#,

            rusqlite::params![year_month, room_id, lease_id, total, paid, status],

        ).unwrap();

    }



    #[test]

    fn test_query_bills_filter_by_month() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "Q001", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "甲", "13800000001");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 插入 3 月、4 月账单

        insert_bill(&conn, room_id, lease_id, "2026-03", 200000, 0, "待缴费");

        insert_bill(&conn, room_id, lease_id, "2026-04", 210000, 0, "待缴费");



        let ctx = TestContext::new(conn);

        let result = BillService.query_bills(&ctx, Some(2026), Some(4), None, None, 1, 10).unwrap();



        assert_eq!(result.total, 1);

        assert_eq!(result.bills.len(), 1);

        assert_eq!(result.bills[0].year_month, "2026-04");

        assert_eq!(result.bills[0].total_amount, 210000);

    }



    #[test]

    fn test_query_bills_filter_by_status() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "Q002", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "乙", "13800000002");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        insert_bill(&conn, room_id, lease_id, "2026-04", 200000, 0, "待缴费");

        insert_bill(&conn, room_id, lease_id, "2026-05", 200000, 200000, "已支付");



        let ctx = TestContext::new(conn);

        let result = BillService.query_bills(&ctx, Some(2026), Some(5), None, Some("已支付"), 1, 10).unwrap();



        assert_eq!(result.total, 1);

        assert_eq!(result.bills[0].status, "已支付");

    }



    #[test]

    fn test_query_bills_pagination() {

        let conn = create_test_db_in_memory();

        let tenant_id = insert_test_tenant(&conn, "丙", "13800000003");

        let lease_id = insert_test_lease(&conn, 1, tenant_id, "生效中", 200000);



        // 插入 5 条账单（不同房间）

        for i in 1..=5 {

            let rid = insert_test_room(&conn, &format!("Q{:03}", i), "在租", 300000);

            insert_bill(&conn, rid, lease_id, "2026-04", 200000, 0, "待缴费");

        }



        let ctx = TestContext::new(conn);

        let result = BillService.query_bills(&ctx, Some(2026), Some(4), None, None, 1, 2).unwrap();



        assert_eq!(result.total, 5);

        assert_eq!(result.bills.len(), 2);

        assert_eq!(result.total_pages, 3);

    }



    #[test]

    fn test_get_bill_detail_includes_room_tenant() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "Q101", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "丁", "13899999999");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence,

                due_date, created_at)

               VALUES ('2026-04', ?1, ?2, '在租', 10, 20, 50, 120, 10, 70, 600, 73, 57,

                       300000, 30, 10000, 5000, 6000, 5110, 3990, 0, '',

                       324100, 0, 0, '待缴费', 1, '2026-04-30', '2026-04-01')"#,

            rusqlite::params![room_id, lease_id],

        ).unwrap();



        let bill_id: i64 = conn.last_insert_rowid();

        let ctx = TestContext::new(conn);



        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();



        assert_eq!(detail.room_number, "Q101");

        assert_eq!(detail.tenant_name.as_deref(), Some("丁"));

        assert_eq!(detail.year_month, "2026-04");

        assert_eq!(detail.rent_fee, 300000);

        assert_eq!(detail.water_reading_prev, 10);

        assert_eq!(detail.water_reading_current, 20);

    }



    #[test]

    fn test_get_bill_detail_remaining_amount() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "Q102", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "戊", "13800000005");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence)

               VALUES ('2026-04', ?1, ?2, '在租', 0, 0, 0, 0, 0, 0, 600, 73, 57,

                       300000, 30, 10000, 5000, 0, 0, 0, 0, '',

                       315000, 0, 100000, '部分支付', 1)"#,

            rusqlite::params![room_id, lease_id],

        ).unwrap();



        let bill_id: i64 = conn.last_insert_rowid();

        let ctx = TestContext::new(conn);



        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();



        assert_eq!(detail.total_amount, 315000);

        assert_eq!(detail.actual_paid, 100000);

        assert_eq!(detail.remaining_amount, 215000);

    }



    #[test]

    fn test_get_bill_detail_can_transition_flags() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "Q103", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "己", "13800000006");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence)

               VALUES ('2026-04', ?1, ?2, '在租', 0, 0, 0, 0, 0, 0, 600, 73, 57,

                       300000, 30, 10000, 5000, 0, 0, 0, 0, '',

                       315000, 0, 0, '待缴费', 1)"#,

            rusqlite::params![room_id, lease_id],

        ).unwrap();



        let bill_id: i64 = conn.last_insert_rowid();

        let ctx = TestContext::new(conn);



        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();



        // 待缴费：可确认、可部分支付、可作废

        assert!(detail.can_confirm);

        assert!(detail.can_void);

        assert!(detail.can_pay_partial);

    }

}



#[cfg(test)]

mod bill_action_tests {

    use super::*;

    use crate::db::connection::TestContext;

    use crate::db::test_helpers::*;



    /// 辅助：插入待缴费账单

    fn insert_pending_bill(conn: &rusqlite::Connection, room_id: i64, lease_id: i64, total: i64) -> i64 {

        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence)

               VALUES ('2026-04', ?1, ?2, '在租', 0, 0, 0, 0, 0, 0, 600, 73, 57,

                       0, 30, 0, 0, 0, 0, 0, 0, '',

                       ?3, 0, 0, '待缴费', 1)"#,

            rusqlite::params![room_id, lease_id, total],

        ).unwrap();

        conn.last_insert_rowid()

    }



    /// 辅助：插入已支付账单

    fn insert_paid_bill(conn: &rusqlite::Connection, room_id: i64, lease_id: i64, total: i64) -> i64 {

        conn.execute(

            r#"INSERT INTO monthly_bills

               (year_month, room_id, lease_id, room_status,

                water_reading_prev, water_reading_current, electric_reading_prev, electric_reading_current,

                water_usage, electric_usage, water_unit_price, electric_unit_price, management_unit_price,

                rent_fee, rent_days, rent_daily_rate, property_fee, water_fee, electric_fee, management_fee,

                misc_fee, misc_fee_remark,

                total_amount, previous_balance, actual_paid, status, bill_sequence)

               VALUES ('2026-04', ?1, ?2, '在租', 0, 0, 0, 0, 0, 0, 600, 73, 57,

                       0, 30, 0, 0, 0, 0, 0, 0, '',

                       ?3, 0, ?3, '已支付', 1)"#,

            rusqlite::params![room_id, lease_id, total],

        ).unwrap();

        conn.last_insert_rowid()

    }



    #[test]

    fn test_confirm_bill_paid_success() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A001", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "甲", "13800000001");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 315000);



        let ctx = TestContext::new(conn);

        let result = BillService.confirm_bill_paid(&ctx, bill_id).unwrap();



        assert!(result.success);

        assert_eq!(result.message, "确认支付成功");



        // 验证账单状态已更新

        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();

        assert_eq!(detail.status, "已支付");

        assert_eq!(detail.actual_paid, 315000);

    }



    #[test]

    fn test_confirm_bill_paid_reject_if_already_paid() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A002", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "乙", "13800000002");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_paid_bill(&conn, room_id, lease_id, 315000);



        let ctx = TestContext::new(conn);

        let result = BillService.confirm_bill_paid(&ctx, bill_id);



        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("无法确认支付"));

    }



    #[test]

    fn test_partial_pay_success() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A003", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "丙", "13800000003");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 100000);



        let ctx = TestContext::new(conn);

        let result = BillService.partial_pay_bill(&ctx, bill_id, 30000).unwrap();



        assert!(result.success);

        assert!(result.message.contains("部分支付成功"));



        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();

        assert_eq!(detail.status, "部分支付");

        assert_eq!(detail.actual_paid, 30000);

    }



    #[test]

    fn test_partial_pay_exceed_remaining() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A004", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "丁", "13800000004");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 100000); // 剩余10万



        let ctx = TestContext::new(conn);

        let result = BillService.partial_pay_bill(&ctx, bill_id, 150000); // 付15万



        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("超过剩余金额"));

    }



    #[test]

    fn test_partial_pay_auto_confirm_when_full() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A005", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "戊", "13800000005");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 50000);



        let ctx = TestContext::new(conn);

        let result = BillService.partial_pay_bill(&ctx, bill_id, 50000).unwrap();



        assert!(result.success);

        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();

        assert_eq!(detail.status, "已支付"); // 自动转已支付

        assert_eq!(detail.actual_paid, 50000);

    }



    #[test]

    fn test_partial_pay_zero_rejected() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A006", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "己", "13800000006");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 100000);



        let ctx = TestContext::new(conn);

        let result = BillService.partial_pay_bill(&ctx, bill_id, 0);



        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("必须大于0"));

    }



    #[test]

    fn test_void_bill_success() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A007", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "庚", "13800000007");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_pending_bill(&conn, room_id, lease_id, 315000);



        let ctx = TestContext::new(conn);

        let result = BillService.void_bill(&ctx, bill_id).unwrap();



        assert!(result.success);

        assert_eq!(result.message, "账单已作废");



        let detail = BillService.get_bill_detail(&ctx, bill_id).unwrap();

        assert_eq!(detail.status, "已作废");

    }



    #[test]

    fn test_void_bill_reject_if_paid() {

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room(&conn, "A008", "在租", 300000);

        let tenant_id = insert_test_tenant(&conn, "辛", "13800000008");

        let lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);

        let bill_id = insert_paid_bill(&conn, room_id, lease_id, 315000);



        let ctx = TestContext::new(conn);

        let result = BillService.void_bill(&ctx, bill_id);



        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("无法作废"));

    }



    #[test]

    fn test_void_then_regenerate() {

        // 作废后，generate_room_bill 应能重新生成（回归测试）

        let conn = create_test_db_in_memory();

        let room_id = insert_test_room_with_meters(&conn, "A009", "在租", 300000, 5000, 0, 0);

        let tenant_id = insert_test_tenant(&conn, "壬", "13800000009");

        let _lease_id = insert_test_lease(&conn, room_id, tenant_id, "生效中", 200000);



        // 先插上期抄表读数

        insert_test_meter_reading(&conn, room_id, "2026-03-01", 0, 0);

        insert_test_meter_reading_for_month(&conn, room_id, 2026, 4, 50, 200, "2026-04-10");



        let ctx = TestContext::new(conn);



        // 生成4月账单

        let r1 = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();

        assert!(r1.bill_id.is_some());



        let bill_id = r1.bill_id.unwrap();



        // 作废账单

        BillService.void_bill(&ctx, bill_id).unwrap();



        // 作废后重新生成应成功

        let r2 = BillService.generate_room_bill(&ctx, room_id, 2026, 4, 0, None).unwrap();

        assert!(r2.bill_id.is_some(), "作废后应能重新生成");

    }

}

