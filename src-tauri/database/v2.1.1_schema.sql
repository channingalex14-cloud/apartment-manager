-- V2.1.1 数据库迁移脚本
-- Phase 4 — 权限系统基础
-- 新增用户表、角色表、权限表

PRAGMA foreign_keys = ON;

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

-- =====================================================
-- 插入默认管理员用户
-- 密码: admin123 (bcrypt hash, 仅供测试使用)
-- =====================================================
INSERT OR IGNORE INTO users (username, password_hash, role, display_name, is_active)
VALUES ('admin', '$2b$12$mSU2A/X0a2o1xlwn61tn0.AtfzqVb7TzNtS62chxmOxGgYjN/55TC', 'admin', '系统管理员', 1);

-- =====================================================
-- 插入默认权限配置
-- =====================================================

-- admin: 所有权限
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

-- frontdesk: 前台操作
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

-- maintenance: 维修人员
INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
('maintenance', 'room', 'rooms', 'read', '房间查看'),
('maintenance', 'room', 'rooms', 'write', '房间状态更新'),
('maintenance', 'lease', 'leases', 'read', '合同查看'),
('maintenance', 'maintenance', 'maintenance', 'manage', '维修管理'),
('maintenance', 'document', 'documents', 'read', '文档查看');

-- finance: 财务人员
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
