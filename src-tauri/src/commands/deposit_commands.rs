use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::interceptors::log_operation;
use crate::models::DepositLedgerResponse;
use crate::require_login;
use crate::services::DepositService;
use std::time::Instant;

#[tauri::command]
pub async fn get_deposit_ledger(
    lease_id: Option<i64>,
    room_id: Option<i64>,
) -> Result<DepositLedgerResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let service = DepositService;
    let result = service
        .get_deposit_ledger(&ctx, lease_id, room_id)
        .map_err(app_error_to_json_string)?;

    log_operation("get_deposit_ledger", None, "deposit", 0, true, start.elapsed().as_millis() as u64);
    Ok(result)
}

#[tauri::command]
pub async fn receive_deposit(
    token: String,
    lease_id: i64,
    amount: i64,
    operator: Option<String>,
    _notes: Option<String>,
) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let service = DepositService;
    service
        .receive_deposit(&ctx, lease_id, amount, &today, operator.as_deref())
        .map_err(app_error_to_json_string)?;

    log_operation("receive_deposit", None, "deposit", lease_id, true, start.elapsed().as_millis() as u64);
    Ok("押金收取成功".to_string())
}

#[tauri::command]
pub async fn refund_deposit(
    token: String,
    lease_id: i64,
    amount: i64,
    operator: Option<String>,
    notes: Option<String>,
) -> Result<String, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = Instant::now();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let reason = notes.as_deref().unwrap_or("正常退房退还押金");
    let service = DepositService;
    service
        .refund_deposit(&ctx, lease_id, amount, reason, &today, operator.as_deref())
        .map_err(app_error_to_json_string)?;

    log_operation("refund_deposit", None, "deposit", lease_id, true, start.elapsed().as_millis() as u64);
    Ok("押金退还成功".to_string())
}
