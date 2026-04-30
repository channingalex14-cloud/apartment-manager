-- V2.0.3 数据库初始化脚本
-- 基于 V2.0.2，版本升至 V2.0.3
-- 变更说明：TOCTOU 漏洞修复不涉及 schema 变更，代码层修复无需迁移脚本
--
-- V2.0.3 代码变更：
-- - #19 generate_single_bill TOCTOU 修复（bill_service.rs）
-- - #20 receive_deposit TOCTOU 修复（deposit_service.rs）
-- - #21 refund_deposit TOCTOU 修复（deposit_service.rs）
-- - #22 测试代码 unwrap() 修复（lease_service.rs）
-- - queries.rs 新增 get_config_value_tx 事务版本接口
-- - queries.rs 顶部新增 TOCTOU 防护规范注释

-- =====================================================
-- 初始化外键约束
-- =====================================================
PRAGMA foreign_keys = ON;

-- =====================================================
-- 表1: 房间表 (rooms)
-- =====================================================
CREATE TABLE IF NOT EXISTS rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_number TEXT NOT NULL UNIQUE,
    floor INTEGER,
    building TEXT DEFAULT '58栋',

    -- 房型：单间、一房一厅、二房一厅、三房一厅，商铺
    room_type TEXT DEFAULT '单间',

    -- 租金配置（单位：分）
    base_rent INTEGER DEFAULT 0,
    property_fee INTEGER DEFAULT 0,
    deposit INTEGER DEFAULT 0,

    -- 状态
    status TEXT DEFAULT '空房'
        CHECK(status IN ('空房','在租','新租','员工','管理','违约','维修中','待清洁')),

    -- 当前水电表读数
    water_meter_current INTEGER DEFAULT 0,
    electric_meter_current INTEGER DEFAULT 0,

    -- 软删除
    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,

    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_rooms_number ON rooms(room_number);
CREATE INDEX IF NOT EXISTS idx_rooms_floor ON rooms(floor);
CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status);
CREATE INDEX IF NOT EXISTS idx_rooms_deleted ON rooms(is_deleted);

-- =====================================================
-- 表2: 租客表 (tenants)
-- =====================================================
CREATE TABLE IF NOT EXISTS tenants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    phone TEXT NOT NULL,
    phone2 TEXT,
    emergency_contact TEXT,
    emergency_phone TEXT,

    -- 软删除
    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,

    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tenants_name ON tenants(name);
CREATE INDEX IF NOT EXISTS idx_tenants_phone ON tenants(phone);
CREATE INDEX IF NOT EXISTS idx_tenants_deleted ON tenants(is_deleted);

-- =====================================================
-- 表3: 合同表 (leases)
-- =====================================================
CREATE TABLE IF NOT EXISTS leases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    tenant_id INTEGER NOT NULL REFERENCES tenants(id),

    contract_number TEXT,
    start_date TEXT NOT NULL,
    end_date TEXT,
    monthly_rent INTEGER DEFAULT 0,
    property_fee INTEGER DEFAULT 0,
    deposit INTEGER DEFAULT 0,

    -- 押金管理（单位：分）
    deposit_received INTEGER DEFAULT 0,
    deposit_balance INTEGER DEFAULT 0,
    deposit_status TEXT DEFAULT '未收取'
        CHECK(deposit_status IN ('未收取','已收取','部分收取','退还','没收')),

    move_in_date TEXT,
    move_out_date TEXT,
    termination_reason TEXT,

    -- 状态
    status TEXT DEFAULT '草稿'
        CHECK(status IN ('草稿','生效中','违约中','待结算','已退房','已作废')),
    status_reason TEXT,

    -- 软删除
    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,

    notes TEXT,

    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_leases_room ON leases(room_id);
CREATE INDEX IF NOT EXISTS idx_leases_tenant ON leases(tenant_id);
CREATE INDEX IF NOT EXISTS idx_leases_status ON leases(status);
CREATE INDEX IF NOT EXISTS idx_leases_dates ON leases(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_leases_deleted ON leases(is_deleted);
CREATE INDEX IF NOT EXISTS idx_leases_tenant_status ON leases(tenant_id, status);

-- =====================================================
-- 表4: 租客历史表 (tenant_history)
-- =====================================================
CREATE TABLE IF NOT EXISTS tenant_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id),
    event_type TEXT NOT NULL
        CHECK(event_type IN ('新入住','换房','换号','退房','恢复')),
    room_id INTEGER REFERENCES rooms(id),
    lease_id INTEGER REFERENCES leases(id),
    event_date TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    notes TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tenant_history_tenant ON tenant_history(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_history_room ON tenant_history(room_id);
CREATE INDEX IF NOT EXISTS idx_tenant_history_date ON tenant_history(event_date);

-- =====================================================
-- 表5: 月度账单表 (monthly_bills)
-- =====================================================
CREATE TABLE IF NOT EXISTS monthly_bills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    year_month TEXT NOT NULL,
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    lease_id INTEGER REFERENCES leases(id),

    lease_start_date TEXT,
    lease_end_date TEXT,
    check_in_day INTEGER,
    check_out_day INTEGER,

    water_reading_prev INTEGER DEFAULT 0,
    water_reading_current INTEGER,
    electric_reading_prev INTEGER DEFAULT 0,
    electric_reading_current INTEGER,

    water_usage INTEGER DEFAULT 0,
    electric_usage INTEGER DEFAULT 0,

    water_unit_price INTEGER DEFAULT 600,
    electric_unit_price INTEGER DEFAULT 73,
    management_unit_price INTEGER DEFAULT 57,

    rent_fee INTEGER DEFAULT 0,
    rent_days INTEGER DEFAULT 0,
    rent_daily_rate INTEGER DEFAULT 0,
    property_fee INTEGER DEFAULT 0,
    water_fee INTEGER DEFAULT 0,
    electric_fee INTEGER DEFAULT 0,
    management_fee INTEGER DEFAULT 0,
    repair_fee INTEGER DEFAULT 0,
    deposit_fee INTEGER DEFAULT 0,

    previous_balance INTEGER DEFAULT 0,
    actual_paid INTEGER DEFAULT 0,

    total_amount INTEGER DEFAULT 0,

    bill_type TEXT DEFAULT '正常'
        CHECK(bill_type IN ('正常','首月免水电','半月结算','末月结算','月中退房结算')),
    room_status TEXT NOT NULL
        CHECK(room_status IN ('在租','新租','员工','管理','违约','空房','退房')),

    status TEXT DEFAULT '待缴费'
        CHECK(status IN ('待缴费','已支付','部分支付','已作废')),
    due_date TEXT,
    paid_date TEXT,

    bill_sequence INTEGER DEFAULT 1,

    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,

    notes TEXT,

    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT,

    UNIQUE(year_month, room_id, bill_sequence)
);

CREATE INDEX IF NOT EXISTS idx_bills_yearmonth ON monthly_bills(year_month);
CREATE INDEX IF NOT EXISTS idx_bills_room ON monthly_bills(room_id);
CREATE INDEX IF NOT EXISTS idx_bills_status ON monthly_bills(status);
CREATE INDEX IF NOT EXISTS idx_bills_type ON monthly_bills(bill_type);
CREATE INDEX IF NOT EXISTS idx_bills_deleted ON monthly_bills(is_deleted);
CREATE INDEX IF NOT EXISTS idx_bills_room_month ON monthly_bills(room_id, year_month);

-- =====================================================
-- 表6: 缴费记录表 (payments)
-- =====================================================
CREATE TABLE IF NOT EXISTS payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bill_id INTEGER REFERENCES monthly_bills(id),
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    lease_id INTEGER REFERENCES leases(id),

    amount INTEGER NOT NULL,
    payment_date TEXT,
    payment_method TEXT
        CHECK(payment_method IN ('微信','支付宝','银行卡','现金','商家码','押金抵扣','混合支付')),

    wechat_amount INTEGER DEFAULT 0,
    alipay_amount INTEGER DEFAULT 0,
    cash_amount INTEGER DEFAULT 0,
    bank_amount INTEGER DEFAULT 0,
    deposit_deduct_amount INTEGER DEFAULT 0,

    payer_name TEXT,
    confirmation_screenshot TEXT,
    operator TEXT,

    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,

    notes TEXT,

    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_payments_bill ON payments(bill_id);
CREATE INDEX IF NOT EXISTS idx_payments_room ON payments(room_id);
CREATE INDEX IF NOT EXISTS idx_payments_date ON payments(payment_date);
CREATE INDEX IF NOT EXISTS idx_payments_method ON payments(payment_method);
CREATE INDEX IF NOT EXISTS idx_payments_deleted ON payments(is_deleted);
CREATE INDEX IF NOT EXISTS idx_payments_room_date ON payments(room_id, payment_date);

-- =====================================================
-- 表7: 押金台账表 (deposit_ledger)
-- =====================================================
CREATE TABLE IF NOT EXISTS deposit_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lease_id INTEGER NOT NULL REFERENCES leases(id),
    room_id INTEGER NOT NULL REFERENCES rooms(id),

    transaction_type TEXT NOT NULL,
    amount INTEGER NOT NULL,
    balance INTEGER NOT NULL,

    reference_bill_id INTEGER,
    reference_payment_id INTEGER,

    operator TEXT,
    transaction_date TEXT,

    notes TEXT,

    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_deposit_lease ON deposit_ledger(lease_id);
CREATE INDEX IF NOT EXISTS idx_deposit_room ON deposit_ledger(room_id);
CREATE INDEX IF NOT EXISTS idx_deposit_type ON deposit_ledger(transaction_type);
CREATE INDEX IF NOT EXISTS idx_deposit_date ON deposit_ledger(transaction_date);
CREATE INDEX IF NOT EXISTS idx_deposit_lease_type ON deposit_ledger(lease_id, transaction_type);

-- =====================================================
-- 表8: 房间状态变更日志表 (room_status_log)
-- =====================================================
CREATE TABLE IF NOT EXISTS room_status_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    lease_id INTEGER REFERENCES leases(id),

    previous_status TEXT,
    new_status TEXT NOT NULL,

    trigger_type TEXT NOT NULL,

    tenant_id INTEGER REFERENCES tenants(id),
    tenant_name TEXT,

    change_date TEXT NOT NULL,
    effective_date TEXT,

    operator TEXT,
    notes TEXT,

    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_statuslog_room ON room_status_log(room_id);
CREATE INDEX IF NOT EXISTS idx_statuslog_date ON room_status_log(change_date);

-- =====================================================
-- 表9: 系统配置表 (system_config)
-- =====================================================
CREATE TABLE IF NOT EXISTS system_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config_key TEXT NOT NULL UNIQUE,
    config_value TEXT,
    config_type TEXT,
    description TEXT,
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

-- 插入默认配置（单位：分）
INSERT OR REPLACE INTO system_config (config_key, config_value, config_type, description) VALUES
('水费单价', '600', 'number', '元/方，600分=6元'),
('电费单价', '73', 'number', '元/度，73分=0.73元'),
('管理费单价', '57', 'number', '元/度，57分=0.57元'),
('水费最低消费', '1200', 'number', '元，1200分=12元'),
('电费最低消费', '700', 'number', '元，700分=7元'),
('管理费最低消费', '300', 'number', '元，300分=3元'),
('公寓名称', '新逸公寓', 'string', ''),
('楼栋地址', '58栋', 'string', ''),
('银行户名', '陈华康', 'string', ''),
('银行账号', '6226 2206 3475 5002', 'string', ''),
('开户行', '民生银行景田支行', 'string', ''),
('默认押金', '200000', 'number', '元，200000分=2000元'),
('默认物业费', '5000', 'number', '元/月，5000分=50元'),
('租金计算方式', '按30天', 'string', '按30天/按实际天数'),
('日期格式', 'YYYY-MM-DD', 'string', '统一日期格式'),
('温馨提示', '请及时缴纳房租水电。逾期收取滞纳金。转账记录请发截图登记', 'string', '');

-- =====================================================
-- 表10: 收费通知单模板表 (notice_templates)
-- =====================================================
CREATE TABLE IF NOT EXISTS notice_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_name TEXT NOT NULL,
    title TEXT DEFAULT '新逸公寓收费通知单',
    header_text TEXT,
    footer_text TEXT DEFAULT '请及时缴纳房租水电。逾期收取滞纳金。转账记录请发截图登记',
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);

INSERT OR REPLACE INTO notice_templates (template_name, is_active) VALUES ('默认模板', 1);

-- =====================================================
-- 表11: 文档存储表 (documents)
-- =====================================================
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    doc_type TEXT NOT NULL,
    original_filename TEXT,
    stored_path TEXT NOT NULL,
    file_size INTEGER DEFAULT 0,
    mime_type TEXT,
    description TEXT,
    uploaded_by TEXT,
    is_deleted INTEGER DEFAULT 0,
    deleted_at TEXT,
    deleted_by TEXT,
    uploaded_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_documents_entity ON documents(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_documents_type ON documents(doc_type);
CREATE INDEX IF NOT EXISTS idx_documents_deleted ON documents(is_deleted);

-- =====================================================
-- 表12: 通知提醒表 (reminders)
-- =====================================================
CREATE TABLE IF NOT EXISTS reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reminder_type TEXT NOT NULL,
    room_id INTEGER REFERENCES rooms(id),
    lease_id INTEGER REFERENCES leases(id),
    title TEXT NOT NULL,
    message TEXT,
    scheduled_date TEXT,
    reminded_at TEXT,
    is_sent INTEGER DEFAULT 0,
    is_read INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_reminders_room ON reminders(room_id);
CREATE INDEX IF NOT EXISTS idx_reminders_date ON reminders(scheduled_date);

-- =====================================================
-- 表13: 操作日志表 (operation_logs)
-- =====================================================
CREATE TABLE IF NOT EXISTS operation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operator TEXT,
    operation_type TEXT NOT NULL,
    entity_type TEXT,
    entity_id INTEGER,
    old_value TEXT,
    new_value TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_logs_entity ON operation_logs(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_logs_operator ON operation_logs(operator);
CREATE INDEX IF NOT EXISTS idx_logs_date ON operation_logs(created_at);

-- =====================================================
-- 表14: 月度汇总缓存表 (monthly_summary_cache)
-- =====================================================
CREATE TABLE IF NOT EXISTS monthly_summary_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    year_month TEXT NOT NULL UNIQUE,

    total_rooms INTEGER DEFAULT 0,
    rented_count INTEGER DEFAULT 0,
    new_rented_count INTEGER DEFAULT 0,
    vacant_count INTEGER DEFAULT 0,
    violation_count INTEGER DEFAULT 0,
    staff_count INTEGER DEFAULT 0,
    management_count INTEGER DEFAULT 0,

    rent_total INTEGER DEFAULT 0,
    property_total INTEGER DEFAULT 0,
    water_total INTEGER DEFAULT 0,
    electric_total INTEGER DEFAULT 0,
    management_total INTEGER DEFAULT 0,
    repair_total INTEGER DEFAULT 0,
    deposit_total INTEGER DEFAULT 0,
    previous_balance_total INTEGER DEFAULT 0,
    actual_paid_total INTEGER DEFAULT 0,

    occupancy_rate REAL DEFAULT 0,

    generated_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_summary_cache_month ON monthly_summary_cache(year_month);

-- =====================================================
-- 初始化 107 个房间数据
-- =====================================================
INSERT INTO rooms (room_number, floor, building, base_rent, property_fee, deposit, status)
SELECT
    printf('%d%02d', floor_num, room_num) as room_number,
    floor_num as floor,
    '58栋' as building,
    200000 as base_rent,      -- 2000元
    5000 as property_fee,      -- 50元
    200000 as deposit,         -- 2000元
    CASE
        WHEN floor_num = 1 AND room_num BETWEEN 1 AND 3 THEN '员工'
        WHEN floor_num IN (2, 3) AND room_num = 1 THEN '管理'
        ELSE '空房'
    END as status
FROM
    (SELECT 1 as floor_num UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 UNION SELECT 5 UNION
     SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9 UNION
     SELECT 10 UNION SELECT 11 UNION SELECT 12 UNION SELECT 13 UNION SELECT 14) floors,
    (SELECT 1 as room_num UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 UNION SELECT 5
     UNION SELECT 6 UNION SELECT 7) rooms
WHERE NOT (floor_num = 1 AND room_num > 3)  -- 1楼只有3个房间
ORDER BY floor_num, room_num;
