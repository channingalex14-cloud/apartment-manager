//! 房间命令

use crate::db::{get_app_context, queries, HasConnection};
use crate::errors::{app_error_to_json_string, AppError};
use crate::interceptors::log_operation;
use crate::models::{RoomResponse, MonthlyBill};
use crate::{require_admin, require_login};
use tracing::info;

/// 列出所有房间
#[tauri::command]
pub async fn list_rooms() -> Result<Vec<RoomResponse>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let rooms = queries::list_rooms(&*conn)
        .map_err(app_error_to_json_string)?;

    log_operation("list_rooms", None, "rooms", 0, true, start.elapsed().as_millis() as u64);
    Ok(rooms)
}

/// 获取房间详情
#[tauri::command]
pub async fn get_room(id: i64) -> Result<Option<RoomResponse>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let room = queries::get_room_response_by_id(&*conn, id)
        .map_err(app_error_to_json_string)?;

    log_operation("get_room", None, "room", id, true, start.elapsed().as_millis() as u64);
    Ok(room)
}

/// 更新房间信息（乐观锁保护）
#[tauri::command]
pub async fn update_room(
    token: String,
    id: i64,
    version: i64,
    base_rent: Option<i64>,
    property_fee: Option<i64>,
    water_meter_current: Option<i64>,
    electric_meter_current: Option<i64>,
    room_type: Option<String>,
) -> Result<bool, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let affected = ctx.transaction(|tx| {
        queries::update_room_with_version_check_tx(
            tx,
            id,
            version,
            base_rent,
            property_fee,
            water_meter_current,
            electric_meter_current,
            room_type.as_deref(),
        )
    }).map_err(app_error_to_json_string)?;

    log_operation("update_room", None, "room", id, affected > 0, start.elapsed().as_millis() as u64);
    info!("更新房间 {}: {} 行受影响", id, affected);

    Ok(affected > 0)
}

/// 直接更新房间状态（用于手动设置特殊状态：维修中/管理/员工等）
#[tauri::command]
pub async fn update_room_status(
    token: String,
    id: i64,
    status: String,
    operator: Option<String>,
    notes: Option<String>,
) -> Result<bool, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let new_status = crate::models::RoomStatus::from_str(&status)
        .map_err(|e| app_error_to_json_string(AppError::InvalidStatus(format!("无效的房间状态 '{}': {}", status, e))))?;

    let affected = ctx.transaction(|tx| {
        crate::services::RoomService::update_room_status(
            tx,
            id,
            new_status,
            operator.as_deref(),
            notes.as_deref(),
        )
    }).map_err(app_error_to_json_string)?;

    log_operation("update_room_status", None, "room", id, affected > 0, start.elapsed().as_millis() as u64);
    info!("更新房间 {} 状态为 {}: {} 行受影响", id, status, affected);

    Ok(affected > 0)
}

/// 获取房间最新水电账单详情
#[tauri::command]
pub async fn get_room_meter_detail(room_id: i64) -> Result<Option<MonthlyBill>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let bill = queries::get_latest_bill_for_room(&*conn, room_id)
        .map_err(app_error_to_json_string)?;
    if let Some(ref b) = bill {
        tracing::info!("[meter_detail] room_id={} year_month={} water_fee={} electric_fee={} management_fee={} water_cur={} electric_cur={}",
            room_id, b.year_month, b.water_fee, b.electric_fee, b.management_fee,
            b.water_reading_current, b.electric_reading_current);
    } else {
        tracing::info!("[meter_detail] room_id={} -> no bill found", room_id);
    }
    Ok(bill)
}
