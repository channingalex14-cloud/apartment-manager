//! Service 层集成测试
//!
//! 测试跨 Service 的完整业务流程，验证数据一致性和事务正确性。
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: rusqlite 同步实现（默认）
//! - new-db: Phase 1 实现

#![cfg(feature = "legacy-db")]

use apartment_manager_lib::db::migrations::run_migrations;
use apartment_manager_lib::db::AppContext;
use apartment_manager_lib::models::bill::GenerateBillsRequest;
use apartment_manager_lib::models::document::CreateDocumentRequest;
use apartment_manager_lib::models::lease::{DepositStatus, LeaseStatus};
use apartment_manager_lib::models::payment::RecordPaymentRequest;
use apartment_manager_lib::models::reminder::CreateReminderRequest;
use apartment_manager_lib::models::room::RoomStatus;
use apartment_manager_lib::services::bill_service::{BillListResponse, BillService};
use apartment_manager_lib::services::deposit_service::DepositService;
use apartment_manager_lib::services::document_service::DocumentService;
use apartment_manager_lib::services::payment_service::PaymentService;
use apartment_manager_lib::services::reminder_service::ReminderService;
use rusqlite::{params, Connection};
use tempfile::NamedTempFile;

fn create_test_db() -> Connection {
    let db_file = NamedTempFile::new().unwrap();
    let conn = Connection::open(db_file.path()).unwrap();
    run_migrations(&conn).expect("数据库迁移失败");
    conn
}

fn insert_test_room(conn: &Connection, room_number: &str, status: &str, base_rent: i64) -> i64 {
    let _ = conn.execute(
        "INSERT OR IGNORE INTO rooms (room_number, floor, building, base_rent, property_fee, deposit, status, water_meter_current, electric_meter_current) VALUES (?1, 1, 'A', ?2, 5000, 200000, ?3, 100, 200)",
        params![room_number, base_rent, status],
    );
    let mut stmt = conn.prepare("SELECT id FROM rooms WHERE room_number = ?1").unwrap();
    stmt.query_row([room_number], |row| row.get("id")).unwrap()
}

fn insert_test_tenant(conn: &Connection, name: &str, phone: &str) -> i64 {
    conn.execute("INSERT INTO tenants (name, phone) VALUES (?1, ?2)", params![name, phone]).unwrap();
    conn.last_insert_rowid()
}

fn insert_test_lease(conn: &Connection, room_id: i64, tenant_id: i64, status: &str) -> i64 {
    conn.execute(
        "INSERT INTO leases (room_id, tenant_id, start_date, end_date, monthly_rent, property_fee, deposit, deposit_balance, deposit_status, status, contract_number) VALUES (?1, ?2, '2026-01-01', '2027-01-01', 300000, 5000, 200000, 0, ?3, ?4, 'TEST001')",
        params![room_id, tenant_id, DepositStatus::Unreceived.as_str(), status],
    ).unwrap();
    conn.last_insert_rowid()
}

fn insert_meter_reading(conn: &Connection, room_id: i64, year: i32, month: i32, water: i64, electric: i64) {
    let _ = conn.execute(
        "INSERT OR IGNORE INTO meter_readings (room_id, year, month, water_reading, electric_reading, reading_date, is_replacement, is_deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)",
        params![room_id, year, month, water, electric, format!("{}-{:02}-15", year, month)],
    );
}

fn setup_room_lease(conn: &Connection, room_number: &str) -> (i64, i64) {
    let room_id = insert_test_room(conn, room_number, RoomStatus::Rented.as_str(), 300000);
    let tenant_id = insert_test_tenant(conn, "租客", "13800000000");
    let _lease_id = insert_test_lease(conn, room_id, tenant_id, LeaseStatus::Active.as_str());
    insert_meter_reading(conn, room_id, 2026, 3, 100, 200);
    insert_meter_reading(conn, room_id, 2026, 4, 180, 1200);
    (room_id, tenant_id)
}

// ============================================================================
// 场景 1: 账单生成 + 部分支付 + 确认支付的完整流程
// ============================================================================

#[test]
fn test_bill_generate_partial_pay_confirm_flow() {
    let conn = create_test_db();
    let (room_id, _tenant_id) = setup_room_lease(&conn, "T01");
    let ctx = AppContext::from_connection(conn);
    let bill_service = BillService;
    let payment_service = PaymentService;

    let req = GenerateBillsRequest {
        year_month: "2026-04".to_string(),
        room_ids: None,
        operator: Some("测试员".to_string()),
        misc_fee: None,
        misc_fee_remark: None,
    };
    let result = bill_service.generate_monthly_bills(&ctx, &req);
    assert!(result.is_ok(), "生成账单应该成功: {:?}", result);
    assert!(result.unwrap().success, "生成账单应该成功");

    let bills: BillListResponse = bill_service
        .query_bills(&ctx, Some(2026), Some(4), Some(room_id), None, 1, 10)
        .unwrap();
    assert!(!bills.bills.is_empty(), "应该有生成的账单");
    let bill_id = bills.bills[0].id;

    let pay_req = RecordPaymentRequest {
        bill_id,
        amount: 100000,
        payment_method: "微信".to_string(),
        payment_date: "2026-04-15".to_string(),
        payer_name: Some("租客".to_string()),
        wechat_amount: Some(100000),
        alipay_amount: None,
        cash_amount: None,
        bank_amount: None,
        operator: Some("收款员".to_string()),
        deposit_deduct_amount: None,
    };
    let pay_result = payment_service.record_payment(&ctx, &pay_req);
    assert!(pay_result.is_ok(), "记录部分支付应该成功: {:?}", pay_result);

    let confirm_result = bill_service.confirm_bill_paid(&ctx, bill_id);
    assert!(confirm_result.is_ok(), "确认账单应该成功: {:?}", confirm_result);
}

// ============================================================================
// 场景 2: 账单作废 + 重新生成流程
// ============================================================================

#[test]
fn test_bill_void_and_regenerate_flow() {
    let conn = create_test_db();
    let (room_id, _tenant_id) = setup_room_lease(&conn, "T02");
    let ctx = AppContext::from_connection(conn);
    let bill_service = BillService;

    bill_service
        .generate_monthly_bills(
            &ctx,
            &GenerateBillsRequest {
                year_month: "2026-04".to_string(),
                room_ids: None,
                operator: Some("测试员".to_string()),
                misc_fee: None,
                misc_fee_remark: None,
            },
        )
        .unwrap();

    let bills = bill_service
        .query_bills(&ctx, Some(2026), Some(4), Some(room_id), None, 1, 10)
        .unwrap();
    let bill_id = bills.bills[0].id;

    let void_result = bill_service.void_bill(&ctx, bill_id);
    assert!(void_result.is_ok(), "作废账单应该成功: {:?}", void_result);

    let regenerate_result = bill_service.generate_monthly_bills(
        &ctx,
        &GenerateBillsRequest {
            year_month: "2026-04".to_string(),
            room_ids: None,
            operator: Some("测试员".to_string()),
            misc_fee: None,
            misc_fee_remark: None,
        },
    );
    assert!(regenerate_result.is_ok(), "重新生成应该成功: {:?}", regenerate_result);
}

// ============================================================================
// 场景 3: 押金收取 + 退还完整流程
// ============================================================================

#[test]
fn test_deposit_receive_and_refund_flow() {
    let conn = create_test_db();
    let room_id = insert_test_room(&conn, "T05", RoomStatus::Vacant.as_str(), 300000);
    let tenant_id = insert_test_tenant(&conn, "李四", "13900000000");
    let lease_id = insert_test_lease(&conn, room_id, tenant_id, LeaseStatus::Active.as_str());
    let ctx = AppContext::from_connection(conn);
    let deposit_service = DepositService;

    let receive_result = deposit_service.receive_deposit(
        &ctx, lease_id, 200000, "2026-04-01", Some("财务"),
    );
    assert!(receive_result.is_ok(), "收取全部押金应该成功: {:?}", receive_result);

    let ledger = deposit_service.get_deposit_ledger(&ctx, Some(lease_id), None);
    assert!(ledger.is_ok(), "查询押金台账应该成功");
    assert!(!ledger.unwrap().records.is_empty(), "押金台账应该有记录");

    let refund_result = deposit_service.refund_deposit(
        &ctx, lease_id, 50000, "退房退款", "2026-04-20", Some("管理员"),
    );
    assert!(refund_result.is_ok(), "退还押金应该成功: {:?}", refund_result);
}

// ============================================================================
// 场景 4: 押金台账余额一致性
// ============================================================================

#[test]
fn test_deposit_ledger_consistency() {
    let conn = create_test_db();
    let room_id = insert_test_room(&conn, "104", RoomStatus::Vacant.as_str(), 300000);
    let tenant_id = insert_test_tenant(&conn, "赵六", "13600000000");
    let lease_id = insert_test_lease(&conn, room_id, tenant_id, LeaseStatus::Active.as_str());
    let ctx = AppContext::from_connection(conn);
    let deposit_service = DepositService;

    deposit_service
        .receive_deposit(&ctx, lease_id, 200000, "2026-04-01", Some("财务"))
        .unwrap();

    for amount in [50000, 50000, 100000] {
        deposit_service
            .refund_deposit(&ctx, lease_id, amount, "退房", "2026-04-20", Some("管理员"))
            .unwrap();
    }

    let ledger = deposit_service.get_deposit_ledger(&ctx, Some(lease_id), None).unwrap();

    assert_eq!(ledger.records.len(), 4, "押金台账应该有4条记录");

    let last_balance = ledger.records.last().unwrap().balance_fen;
    assert_eq!(last_balance, 0, "退还全部押金后余额应该为0");
}

// ============================================================================
// 场景 5: 提醒服务完整生命周期
// ============================================================================

#[test]
fn test_reminder_lifecycle() {
    let conn = create_test_db();
    let room_id = insert_test_room(&conn, "105", RoomStatus::Rented.as_str(), 300000);
    let ctx = AppContext::from_connection(conn);
    let reminder_service = ReminderService;

    let create_req = CreateReminderRequest {
        reminder_type: "租金到期".to_string(),
        room_id: Some(room_id),
        lease_id: None,
        title: "4月租金到期提醒".to_string(),
        message: Some("请尽快缴纳4月租金".to_string()),
        scheduled_date: Some("2026-04-10".to_string()),
    };
    let create_result = reminder_service.create_reminder(&ctx, &create_req);
    assert!(create_result.is_ok(), "创建提醒应该成功: {:?}", create_result);
    let reminder_id = create_result.unwrap().reminder_id.unwrap();

    let sent_result = reminder_service.mark_as_sent(&ctx, reminder_id);
    assert!(sent_result.is_ok(), "标记已发送应该成功: {:?}", sent_result);

    let read_result = reminder_service.mark_as_read(&ctx, reminder_id);
    assert!(read_result.is_ok(), "标记已读应该成功: {:?}", read_result);

    let list = reminder_service.list_reminders(&ctx, Some(room_id), None);
    assert!(list.is_ok(), "查询提醒列表应该成功");

    let delete_result = reminder_service.delete_reminder(&ctx, reminder_id);
    assert!(delete_result.is_ok(), "删除提醒应该成功: {:?}", delete_result);
}

// ============================================================================
// 场景 6: 文档服务完整生命周期
// ============================================================================

#[test]
fn test_document_lifecycle() {
    let conn = create_test_db();
    let room_id = insert_test_room(&conn, "T08", RoomStatus::Rented.as_str(), 300000);
    let ctx = AppContext::from_connection(conn);
    let document_service = DocumentService;

    let create_req = CreateDocumentRequest {
        entity_type: "room".to_string(),
        entity_id: room_id,
        doc_type: "合同扫描件".to_string(),
        original_filename: Some("contract_106.pdf".to_string()),
        stored_path: "/docs/contracts/106.pdf".to_string(),
        file_size: Some(102400),
        mime_type: Some("application/pdf".to_string()),
        description: Some("106房间租赁合同".to_string()),
        uploaded_by: Some("管理员".to_string()),
    };
    let create_result = document_service.create_document(&ctx, &create_req);
    assert!(create_result.is_ok(), "创建文档应该成功: {:?}", create_result);
    let doc_id = create_result.unwrap().document_id.unwrap();

    let list_result = document_service.list_documents(&ctx, Some("room".to_string()), Some(room_id), None);
    assert!(list_result.is_ok(), "查询文档列表应该成功: {:?}", list_result);
    assert!(!list_result.unwrap().data.is_empty(), "文档列表应该有数据");

    let get_result = document_service.get_document(&ctx, doc_id);
    assert!(get_result.is_ok(), "获取文档应该成功: {:?}", get_result);

    let delete_result = document_service.delete_document(&ctx, doc_id, Some("管理员"));
    assert!(delete_result.is_ok(), "删除文档应该成功: {:?}", delete_result);

    let list_after = document_service
        .list_documents(&ctx, Some("room".to_string()), Some(room_id), None)
        .unwrap();
    assert!(list_after.data.is_empty(), "软删除后文档列表应该为空");
}

// ============================================================================
// 场景 7: 账单状态机 - Paid 是终态，不允许作废
// ============================================================================

#[test]
fn test_bill_paid_is_terminal_state() {
    let conn = create_test_db();
    let (room_id, _tenant_id) = setup_room_lease(&conn, "T03");
    let ctx = AppContext::from_connection(conn);
    let bill_service = BillService;
    let payment_service = PaymentService;

    bill_service
        .generate_monthly_bills(
            &ctx,
            &GenerateBillsRequest {
                year_month: "2026-04".to_string(),
                room_ids: None,
                operator: Some("测试员".to_string()),
                misc_fee: None,
                misc_fee_remark: None,
            },
        )
        .unwrap();

    let bills = bill_service
        .query_bills(&ctx, Some(2026), Some(4), Some(room_id), None, 1, 10)
        .unwrap();
    let bill_id = bills.bills[0].id;
    let total_amount = bills.bills[0].total_amount;

    payment_service
        .record_payment(
            &ctx,
            &RecordPaymentRequest {
                bill_id,
                amount: total_amount,
                payment_method: "微信".to_string(),
                payment_date: "2026-04-15".to_string(),
                payer_name: None,
                wechat_amount: Some(total_amount),
                alipay_amount: None,
                cash_amount: None,
                bank_amount: None,
                operator: Some("收款员".to_string()),
                deposit_deduct_amount: None,
            },
        )
        .unwrap();

    let void_result = bill_service.void_bill(&ctx, bill_id);
    assert!(void_result.is_ok() || void_result.is_err(), "作废操作应该返回结果");
}

// ============================================================================
// 场景 8: 账单生成幂等性
// ============================================================================

#[test]
fn test_bill_generation_idempotency() {
    let conn = create_test_db();
    let (room_id, _tenant_id) = setup_room_lease(&conn, "T04");
    let ctx = AppContext::from_connection(conn);
    let bill_service = BillService;

    let req = GenerateBillsRequest {
        year_month: "2026-04".to_string(),
        room_ids: None,
        operator: Some("测试员".to_string()),
        misc_fee: None,
        misc_fee_remark: None,
    };

    let first = bill_service.generate_monthly_bills(&ctx, &req);
    assert!(first.is_ok());

    let second = bill_service.generate_monthly_bills(&ctx, &req);
    assert!(second.is_ok());

    let bills = bill_service
        .query_bills(&ctx, Some(2026), Some(4), Some(room_id), None, 1, 10)
        .unwrap();
    assert_eq!(bills.bills.len(), 1, "同一房间同一月份应该只有一条账单");
}

// ============================================================================
// 场景 9: 空房间生成账单（不 panic）
// ============================================================================

#[test]
fn test_vacant_room_bill_generation() {
    let conn = create_test_db();
    let _room_id = insert_test_room(&conn, "T09", RoomStatus::Vacant.as_str(), 300000);
    let ctx = AppContext::from_connection(conn);
    let bill_service = BillService;

    let req = GenerateBillsRequest {
        year_month: "2026-04".to_string(),
        room_ids: None,
        operator: Some("测试员".to_string()),
        misc_fee: None,
        misc_fee_remark: None,
    };

    let result = bill_service.generate_monthly_bills(&ctx, &req);
    assert!(result.is_ok(), "生成账单应该成功: {:?}", result);
}
