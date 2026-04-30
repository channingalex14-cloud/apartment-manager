//! 新逸公寓管理系统 V2.0.5
//!
//! 主入口
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: 完整应用（默认）
//! - new-db: 仅编译 lib，不编译 main（Phase 1）

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "legacy-db")]
use std::path::PathBuf;
#[cfg(feature = "legacy-db")]
use tauri::Manager;

#[cfg(feature = "legacy-db")]
fn main() {
    apartment_manager_lib::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 获取应用数据目录并初始化数据库路径
            let app_dir = app.path().app_data_dir()
                .expect("无法获取应用数据目录");
            std::fs::create_dir_all(&app_dir)
                .expect("无法创建应用数据目录");
            let db_path: PathBuf = app_dir.join("apartment.db");
            apartment_manager_lib::db::init_db(db_path)
                .expect("无法初始化数据库路径");

            // 初始化数据库表结构
            let conn = apartment_manager_lib::db::create_connection()
                .expect("无法创建数据库连接");
            apartment_manager_lib::db::run_migrations(&conn)
                .expect("无法运行数据库迁移");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 房间命令
            apartment_manager_lib::commands::room_commands::list_rooms,
            apartment_manager_lib::commands::room_commands::get_room,
            apartment_manager_lib::commands::room_commands::update_room,
            apartment_manager_lib::commands::room_commands::update_room_status,
            apartment_manager_lib::commands::room_commands::get_room_meter_detail,
            // 租客命令
            apartment_manager_lib::commands::tenant_commands::list_tenants,
            apartment_manager_lib::commands::tenant_commands::get_tenant,
            apartment_manager_lib::commands::tenant_commands::create_tenant,
            apartment_manager_lib::commands::tenant_commands::update_tenant,
            apartment_manager_lib::commands::tenant_commands::get_tenant_history,
            apartment_manager_lib::commands::tenant_commands::delete_tenant,
            // 合同命令
            apartment_manager_lib::commands::lease_commands::list_leases,
            apartment_manager_lib::commands::lease_commands::get_lease,
            apartment_manager_lib::commands::lease_commands::create_lease,
            apartment_manager_lib::commands::lease_commands::activate_lease,
            apartment_manager_lib::commands::lease_commands::check_in,
            apartment_manager_lib::commands::lease_commands::check_out,
            apartment_manager_lib::commands::lease_commands::mark_violation,
            apartment_manager_lib::commands::lease_commands::recover_from_violation,
            // 账单命令
            apartment_manager_lib::commands::bill_commands::list_bills,
            apartment_manager_lib::commands::bill_commands::generate_monthly_bills,
            apartment_manager_lib::commands::bill_commands::query_bills,
            apartment_manager_lib::commands::bill_commands::get_bill_detail,
            apartment_manager_lib::commands::bill_commands::confirm_bill_paid,
            apartment_manager_lib::commands::bill_commands::partial_pay_bill,
            apartment_manager_lib::commands::bill_commands::void_bill,
            apartment_manager_lib::commands::bill_commands::archive_bills,
            apartment_manager_lib::commands::bill_commands::restore_bills,
            apartment_manager_lib::commands::bill_commands::list_archived_months,
            apartment_manager_lib::commands::bill_commands::get_bill_summary,
            // 缴费命令
            apartment_manager_lib::commands::payment_commands::list_payments,
            apartment_manager_lib::commands::payment_commands::record_payment,
            apartment_manager_lib::commands::payment_commands::void_payment,
            apartment_manager_lib::commands::payment_commands::update_payment_method,
            // 配置命令
            apartment_manager_lib::commands::config_commands::get_config,
            apartment_manager_lib::commands::config_commands::set_config,
            // 抄表命令
            apartment_manager_lib::commands::meter_reading_commands::record_meter_reading,
            apartment_manager_lib::commands::meter_reading_commands::batch_record_meter_readings,
            // 押金命令
            apartment_manager_lib::commands::deposit_commands::get_deposit_ledger,
            apartment_manager_lib::commands::deposit_commands::receive_deposit,
            apartment_manager_lib::commands::deposit_commands::refund_deposit,
            // 提醒命令
            apartment_manager_lib::commands::reminder_commands::create_reminder,
            apartment_manager_lib::commands::reminder_commands::list_reminders,
            apartment_manager_lib::commands::reminder_commands::get_pending_reminders,
            apartment_manager_lib::commands::reminder_commands::update_reminder_status,
            apartment_manager_lib::commands::reminder_commands::mark_reminder_sent,
            apartment_manager_lib::commands::reminder_commands::mark_reminder_read,
            apartment_manager_lib::commands::reminder_commands::delete_reminder,
            // 文档命令
            apartment_manager_lib::commands::document_commands::create_document,
            apartment_manager_lib::commands::document_commands::list_documents,
            apartment_manager_lib::commands::document_commands::get_document,
            apartment_manager_lib::commands::document_commands::delete_document,
            apartment_manager_lib::commands::document_commands::get_document_count,
            // 报表命令
            apartment_manager_lib::commands::report_commands::generate_monthly_summary,
            apartment_manager_lib::commands::report_commands::get_summary_report,
            apartment_manager_lib::commands::report_commands::list_summary_reports,
            // 维护命令
            apartment_manager_lib::commands::maintenance_commands::vacuum_database_cmd,
            // 导入命令
            apartment_manager_lib::commands::import_commands::import_monthly_bills,
            apartment_manager_lib::commands::diagnostic::diagnose_excel_file,
            apartment_manager_lib::commands::diagnostic::diagnose_database,
            apartment_manager_lib::commands::diagnostic::diagnose_database_text,
            apartment_manager_lib::commands::diagnostic::diagnose_room_detail,
            apartment_manager_lib::commands::diagnostic::fix_management_rooms,
            apartment_manager_lib::commands::diagnostic::diagnose_meter_bill,
            apartment_manager_lib::commands::diagnostic::fix_meter_fees,
            // 备份命令
            apartment_manager_lib::commands::backup_commands::backup_database,
            apartment_manager_lib::commands::backup_commands::list_backups,
            apartment_manager_lib::commands::backup_commands::restore_backup,
            apartment_manager_lib::commands::backup_commands::delete_backup,
            apartment_manager_lib::commands::backup_commands::get_backup_settings,
            apartment_manager_lib::commands::backup_commands::save_backup_settings,
            // 导出命令
            apartment_manager_lib::commands::export_commands::export_data,
            // 认证命令
            apartment_manager_lib::commands::auth_commands::login,
            apartment_manager_lib::commands::auth_commands::logout,
            apartment_manager_lib::commands::auth_commands::get_current_user,
            apartment_manager_lib::commands::auth_commands::list_users,
            apartment_manager_lib::commands::auth_commands::check_permission,
            apartment_manager_lib::commands::auth_commands::create_user,
            apartment_manager_lib::commands::auth_commands::update_user,
            apartment_manager_lib::commands::auth_commands::reset_password,
            apartment_manager_lib::commands::auth_commands::delete_user,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用时发生错误");
}

#[cfg(feature = "new-db")]
use std::path::PathBuf;
#[cfg(feature = "new-db")]
use tauri::Manager;

#[cfg(feature = "new-db")]
fn main() {
    apartment_manager_lib::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()
                .expect("无法获取应用数据目录");
            std::fs::create_dir_all(&app_dir)
                .expect("无法创建应用数据目录");
            let db_path: PathBuf = app_dir.join("apartment.db");

            tauri::async_runtime::block_on(async {
                apartment_manager_lib::db::init_db_async(db_path)
                    .await
                    .expect("无法初始化异步数据库连接池");

                let pool = apartment_manager_lib::db::get_pool()
                    .expect("无法获取数据库连接池");
                apartment_manager_lib::db::run_migrations(pool)
                    .await
                    .expect("无法运行数据库迁移");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Phase 4: Command 层迁移完成后逐步添加
        ])
        .run(tauri::generate_context!())
        .expect("启动应用时发生错误");
}
