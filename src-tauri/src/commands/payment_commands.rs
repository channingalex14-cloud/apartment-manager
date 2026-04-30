//! 缴费命令

use crate::db::{get_app_context, queries};
use crate::errors::app_error_to_json_string;
use crate::interceptors::log_operation;
use crate::models::{Payment, PaymentResponse, RecordPaymentRequest};
use crate::{require_admin, require_login};
use crate::services::PaymentService;

/// 列出缴费记录
#[tauri::command]
pub async fn list_payments(bill_id: Option<i64>) -> Result<Vec<Payment>, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let conn = ctx.get_conn().map_err(app_error_to_json_string)?;
    let payments = queries::list_payments(&*conn, bill_id)
        .map_err(app_error_to_json_string)?;

    log_operation("list_payments", None, "payments", 0, true, start.elapsed().as_millis() as u64);
    Ok(payments)
}

/// 记录缴费
#[tauri::command]
pub async fn record_payment(token: String, req: RecordPaymentRequest) -> Result<PaymentResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let service = PaymentService;
    let result = service.record_payment(&ctx, &req).map_err(app_error_to_json_string)?;

    log_operation("record_payment", req.operator.clone(), "payment", result.payment_id.unwrap_or(0), result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 作废缴费记录（红冲）
#[tauri::command]
pub async fn void_payment(token: String, payment_id: i64, operator: Option<String>) -> Result<PaymentResponse, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let service = PaymentService;
    let result = service.void_payment(&ctx, payment_id, operator.as_deref())
        .map_err(app_error_to_json_string)?;

    log_operation("void_payment", operator, "payment", payment_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}

/// 更新缴费记录的付款方式
#[tauri::command]
pub async fn update_payment_method(token: String, payment_id: i64, payment_method: String, operator: Option<String>) -> Result<PaymentResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let start = std::time::Instant::now();

    let service = PaymentService;
    let result = service.update_payment_method(&ctx, payment_id, &payment_method, operator.as_deref())
        .map_err(app_error_to_json_string)?;

    log_operation("update_payment_method", operator, "payment", payment_id, result.success, start.elapsed().as_millis() as u64);
    Ok(result)
}
