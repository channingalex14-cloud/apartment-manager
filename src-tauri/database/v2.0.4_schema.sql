-- V2.0.4 数据库初始化脚本
-- 基于 V2.0.3，版本升至 V2.0.4
-- 变更说明：Phase 2 — 新增 meter_readings 表，支持独立抄表录入
--
-- V2.0.4 变更：
-- - 新增 meter_readings 表（水电表读数独立存储，支持先抄表后出账）
-- - generate_room_bill 改从 meter_readings 读取本期读数
-- - 账单生成必须有对应的抄表记录，否则跳过

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

    -- 乐观锁版本号
    version INTEGER NOT NULL DEFAULT 0,

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
    misc_fee INTEGER NOT NULL DEFAULT 0,
    misc_fee_remark TEXT NOT NULL DEFAULT '',
    is_archived INTEGER NOT NULL DEFAULT 0,
    archived_at TEXT,

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
    is_deleted INTEGER NOT NULL DEFAULT 0,
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
-- 初始化 107 个房间数据（根据 Excel 实际结构）
-- =====================================================
INSERT INTO rooms (room_number, floor, building, base_rent, property_fee, deposit, status)
VALUES
    -- 1楼: 101 (1个)
    ('101', 1, '58栋', 200000, 5000, 200000, '空房'),
    -- 2楼: 201-204 (4个)
    ('201', 2, '58栋', 200000, 5000, 200000, '空房'),
    ('202', 2, '58栋', 200000, 5000, 200000, '空房'),
    ('203', 2, '58栋', 200000, 5000, 200000, '空房'),
    ('204', 2, '58栋', 200000, 5000, 200000, '空房'),
    -- 3楼: 301-302 (2个)
    ('301', 3, '58栋', 200000, 5000, 200000, '空房'),
    ('302', 3, '58栋', 200000, 5000, 200000, '空房'),
    -- 4楼: 401-410 (10个)
    ('401', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('402', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('403', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('404', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('405', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('406', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('407', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('408', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('409', 4, '58栋', 200000, 5000, 200000, '空房'),
    ('410', 4, '58栋', 200000, 5000, 200000, '空房'),
    -- 5楼: 501-510 (10个)
    ('501', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('502', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('503', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('504', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('505', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('506', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('507', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('508', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('509', 5, '58栋', 200000, 5000, 200000, '空房'),
    ('510', 5, '58栋', 200000, 5000, 200000, '空房'),
    -- 6楼: 601-610 (10个)
    ('601', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('602', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('603', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('604', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('605', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('606', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('607', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('608', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('609', 6, '58栋', 200000, 5000, 200000, '空房'),
    ('610', 6, '58栋', 200000, 5000, 200000, '空房'),
    -- 7楼: 701-710 (10个)
    ('701', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('702', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('703', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('704', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('705', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('706', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('707', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('708', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('709', 7, '58栋', 200000, 5000, 200000, '空房'),
    ('710', 7, '58栋', 200000, 5000, 200000, '空房'),
    -- 8楼: 801-810 (10个)
    ('801', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('802', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('803', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('804', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('805', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('806', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('807', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('808', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('809', 8, '58栋', 200000, 5000, 200000, '空房'),
    ('810', 8, '58栋', 200000, 5000, 200000, '空房'),
    -- 9楼: 901-910 (10个)
    ('901', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('902', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('903', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('904', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('905', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('906', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('907', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('908', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('909', 9, '58栋', 200000, 5000, 200000, '空房'),
    ('910', 9, '58栋', 200000, 5000, 200000, '空房'),
    -- 10楼: 1001-1010 (10个)
    ('1001', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1002', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1003', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1004', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1005', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1006', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1007', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1008', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1009', 10, '58栋', 200000, 5000, 200000, '空房'),
    ('1010', 10, '58栋', 200000, 5000, 200000, '空房'),
    -- 11楼: 1101-1109 (9个)
    ('1101', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1102', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1103', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1104', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1105', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1106', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1107', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1108', 11, '58栋', 200000, 5000, 200000, '空房'),
    ('1109', 11, '58栋', 200000, 5000, 200000, '空房'),
    -- 12楼: 1201-1209 (9个)
    ('1201', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1202', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1203', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1204', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1205', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1206', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1207', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1208', 12, '58栋', 200000, 5000, 200000, '空房'),
    ('1209', 12, '58栋', 200000, 5000, 200000, '空房'),
    -- 13楼: 1301-1309 (9个)
    ('1301', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1302', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1303', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1304', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1305', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1306', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1307', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1308', 13, '58栋', 200000, 5000, 200000, '空房'),
    ('1309', 13, '58栋', 200000, 5000, 200000, '空房'),
    -- 14楼: 1401-1403 (3个)
    ('1401', 14, '58栋', 200000, 5000, 200000, '空房'),
    ('1402', 14, '58栋', 200000, 5000, 200000, '空房'),
    ('1403', 14, '58栋', 200000, 5000, 200000, '空房');

-- =====================================================
-- 表15: 水电表读数记录表 (meter_readings)
-- =====================================================
CREATE TABLE IF NOT EXISTS meter_readings (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id          INTEGER NOT NULL,
    year             INTEGER NOT NULL,
    month            INTEGER NOT NULL,
    water_reading    INTEGER NOT NULL DEFAULT 0,
    electric_reading INTEGER NOT NULL DEFAULT 0,
    reading_date     TEXT NOT NULL,
    operator         TEXT,
    is_replacement   INTEGER NOT NULL DEFAULT 0,
    is_deleted       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT DEFAULT (datetime('now')),
    updated_at       TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (room_id) REFERENCES rooms(id),
    UNIQUE (room_id, year, month)
);

CREATE INDEX IF NOT EXISTS idx_meter_readings_room ON meter_readings(room_id);
CREATE INDEX IF NOT EXISTS idx_meter_readings_year_month ON meter_readings(year, month);
CREATE INDEX IF NOT EXISTS idx_meter_readings_deleted ON meter_readings(is_deleted);

-- =====================================================
-- 表16: 用户表 (users)
-- =====================================================
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'frontdesk'
        CHECK(role IN ('admin', 'frontdesk', 'maintenance', 'finance')),
    display_name TEXT,
    is_active INTEGER DEFAULT 1,
    last_login_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);

-- =====================================================
-- 表17: 权限表 (permissions)
-- =====================================================
CREATE TABLE IF NOT EXISTS permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    role TEXT NOT NULL
        CHECK(role IN ('admin', 'frontdesk', 'maintenance', 'finance')),
    permission TEXT NOT NULL,
    resource TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'read'
        CHECK(action IN ('read', 'write', 'manage', 'none')),
    description TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(role, permission, resource)
);

CREATE INDEX IF NOT EXISTS idx_permissions_role ON permissions(role);
CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource);

-- 插入默认管理员（密码：admin123）
INSERT OR IGNORE INTO users (username, password_hash, role, display_name, is_active)
VALUES ('admin', '$2b$12$mSU2A/X0a2o1xlwn61tn0.AtfzqVb7TzNtS62chxmOxGgYjN/55TC', 'admin', '系统管理员', 1);

-- admin 全权限
INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
 ('admin', 'room', 'rooms', 'manage', '房间管理'),
 ('admin', 'lease', 'leases', 'manage', '合同管理'),
 ('admin', 'tenant', 'tenants', 'manage', '租客管理'),
 ('admin', 'bill', 'bills', 'manage', '账单管理'),
 ('admin', 'payment', 'payments', 'manage', '缴费管理'),
 ('admin', 'deposit', 'deposits', 'manage', '押金管理'),
 ('admin', 'config', 'system_config', 'manage', '系统配置'),
 ('admin', 'report', 'reports', 'read', '报表查看'),
 ('admin', 'document', 'documents', 'manage', '文档管理'),
 ('admin', 'excel', 'excel', 'manage', 'Excel导入导出'),
 ('admin', 'backup', 'backup', 'manage', '备份恢复'),
 ('admin', 'user', 'users', 'manage', '用户管理');

-- frontdesk 权限
INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
 ('frontdesk', 'room', 'rooms', 'read', '房间查看'),
 ('frontdesk', 'lease', 'leases', 'read', '合同查看'),
 ('frontdesk', 'tenant', 'tenants', 'manage', '租客管理'),
 ('frontdesk', 'bill', 'bills', 'read', '账单查看'),
 ('frontdesk', 'bill', 'bills', 'write', '账单操作'),
 ('frontdesk', 'payment', 'payments', 'manage', '缴费管理'),
 ('frontdesk', 'deposit', 'deposits', 'read', '押金查看'),
 ('frontdesk', 'report', 'reports', 'read', '报表查看'),
 ('frontdesk', 'document', 'documents', 'read', '文档查看'),
 ('frontdesk', 'excel', 'excel', 'read', 'Excel导出');

-- maintenance 权限
INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
 ('maintenance', 'room', 'rooms', 'read', '房间查看'),
 ('maintenance', 'room', 'rooms', 'write', '房间状态更新'),
 ('maintenance', 'lease', 'leases', 'read', '合同查看'),
 ('maintenance', 'maintenance', 'maintenance', 'manage', '维修管理'),
 ('maintenance', 'document', 'documents', 'read', '文档查看');

-- finance 权限
INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
 ('finance', 'room', 'rooms', 'read', '房间查看'),
 ('finance', 'lease', 'leases', 'read', '合同查看'),
 ('finance', 'tenant', 'tenants', 'read', '租客查看'),
 ('finance', 'bill', 'bills', 'manage', '账单管理'),
 ('finance', 'payment', 'payments', 'manage', '缴费管理'),
 ('finance', 'deposit', 'deposits', 'manage', '押金管理'),
 ('finance', 'report', 'reports', 'manage', '报表管理'),
 ('finance', 'excel', 'excel', 'manage', 'Excel导入导出'),
 ('finance', 'config', 'system_config', 'read', '配置查看');
