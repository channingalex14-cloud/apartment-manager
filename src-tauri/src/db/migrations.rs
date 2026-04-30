//! 数据库迁移
//!
//! 表结构初始化和版本管理

use rusqlite::{Connection, Result as SqliteResult};
use tracing::info;

const CURRENT_VERSION: &str = "2.1.2";
const INIT_SQL: &str = include_str!("../../database/v2.0.4_schema.sql");

/// 获取当前数据库版本
pub fn get_schema_version(conn: &Connection) -> String {
    // 检查版本表是否存在
    let table_exists = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'")
        .ok()
        .map(|mut stmt| stmt.exists([]).unwrap_or(false))
        .unwrap_or(false);

    if !table_exists {
        return "0.0.0".to_string();
    }

    conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY applied_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "0.0.0".to_string())
}

/// 比较版本号（a < b 返回 true）
fn version_less_than(a: &str, b: &str) -> bool {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        if a_part < b_part {
            return true;
        }
        if a_part > b_part {
            return false;
        }
    }
    a_parts.len() < b_parts.len()
}

/// 执行数据库迁移
pub fn run_migrations(conn: &Connection) -> SqliteResult<()> {
    let current_version = get_schema_version(conn);
    info!("当前数据库版本: {}, 目标版本: {}", current_version, CURRENT_VERSION);

    // 如果已经是最新的，无需迁移
    if current_version == CURRENT_VERSION {
        info!("数据库已是最新版本");
        return Ok(());
    }

    // 如果版本高于当前版本（不应该发生），报错
    if !version_less_than(&current_version, CURRENT_VERSION) {
        return Err(rusqlite::Error::InvalidQuery);
    }

    info!("执行数据库迁移从 {} 到 {}", current_version, CURRENT_VERSION);

    // 创建版本表（如果不存在）
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            applied_at TEXT DEFAULT (datetime('now'))
        );",
    )?;

    // 执行增量迁移（全部成功后才更新版本号）
    run_incremental_migrations(conn, &current_version)?;

    // 验证新增列确实存在（防止 execute() 对 DDL 静默成功的情况）
    // 只有版本 >= 2.0.0 且 < 2.0.6 时才检查 misc_fee
    // 只有版本 >= 2.0.0 且 < 2.0.8 时才检查 repair_fee
    // 注意：0.0.0 数据库也会执行增量迁移（见上方 if from_version == "0.0.0" 分支），
    // 但 INIT_SQL 为 v2.0.4 已包含 misc_fee 和 repair_fee，因此跳过验证检查
    let needs_misc_check = current_version != "0.0.0" && version_less_than(&current_version, "2.0.6");
    if needs_misc_check {
        let misc_ok = conn
            .prepare("PRAGMA table_info(monthly_bills)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "misc_fee" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);
        if !misc_ok {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    let needs_repair_check = current_version != "0.0.0" && version_less_than(&current_version, "2.0.8");
    if needs_repair_check {
        let repair_ok = conn
            .prepare("PRAGMA table_info(monthly_bills)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "repair_fee" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);
        if !repair_ok {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    // 记录版本
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [CURRENT_VERSION],
    )?;

    info!("数据库迁移完成，新版本: {}", CURRENT_VERSION);
    Ok(())
}

/// 执行增量迁移
fn run_incremental_migrations(conn: &Connection, from_version: &str) -> SqliteResult<()> {
    // 从 0.0.0 迁移到任意版本都需要初始化数据库结构（v2.0.4 schema）
    // 但仍需继续执行后续增量迁移（2.0.5+）以添加新字段（如 misc_fee, repair_fee）
    if from_version == "0.0.0" {
        info!("执行初始数据迁移...");
        conn.execute_batch(INIT_SQL)?;
        // 不要 return！继续执行后续增量迁移
    }

    // 从 2.0.1 或更早版本迁移到 2.0.2
    if version_less_than(from_version, "2.0.2") {
        info!("添加 room_type 字段到 rooms 表...");
        // 检查字段是否已存在（需要遍历列名）
        let has_room_type = conn
            .prepare("PRAGMA table_info(rooms)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "room_type" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_room_type {
            conn.execute_batch(
                "ALTER TABLE rooms ADD COLUMN room_type TEXT DEFAULT '单间';",
            )?;
        }
    }

    // V2.0.3：代码层 TOCTOU 修复，无 schema 变更，无需迁移操作
    // 2.0.2 → 2.0.3 增量迁移为空（版本号在 run_migrations 统一更新）

    // V2.0.4：Phase 2 — 新增 meter_readings 表，支持独立抄表录入
    if version_less_than(from_version, "2.0.4") {
        info!("创建 meter_readings 表...");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meter_readings (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                room_id          INTEGER NOT NULL,
                year             INTEGER NOT NULL,
                month            INTEGER NOT NULL,
                water_reading    INTEGER NOT NULL DEFAULT 0,
                electric_reading  INTEGER NOT NULL DEFAULT 0,
                reading_date     TEXT NOT NULL,
                operator         TEXT,
                is_replacement   INTEGER NOT NULL DEFAULT 0,
                is_deleted       INTEGER NOT NULL DEFAULT 0,
                created_at       TEXT DEFAULT (datetime('now')),
                updated_at       TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (room_id) REFERENCES rooms(id),
                UNIQUE (room_id, year, month)
            );",
        )?;
    }

    // 兜底：确保 room_type 列存在（处理旧数据库迁移时遗漏的情况）
    {
        let has_room_type = conn
            .prepare("PRAGMA table_info(rooms)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "room_type" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_room_type {
            info!("[migration] 发现旧数据库缺少 room_type 列，执行补充迁移...");
            conn.execute_batch(
                "ALTER TABLE rooms ADD COLUMN room_type TEXT DEFAULT '单间';",
            )?;
        }
    }

    // V2.0.5：修复 reminders 表软删除支持
    if version_less_than(from_version, "2.0.5") {
        info!("添加 is_deleted 列到 reminders 表...");
        match conn.execute(
            "ALTER TABLE reminders ADD COLUMN is_deleted INTEGER DEFAULT 0",
            [],
        ) {
            Ok(_) => info!("[migration] reminders.is_deleted 列添加成功"),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("duplicate column") {
                    info!("[migration] reminders.is_deleted 列已存在，跳过");
                } else {
                    return Err(e);
                }
            }
        }
    }

    // V2.0.6：新增 misc_fee + misc_fee_remark 字段支持收费通知单灵活杂费
    if version_less_than(from_version, "2.0.6") {
        info!("添加 misc_fee 和 misc_fee_remark 字段到 monthly_bills 表...");

        // 检查列是否已存在
        let has_misc_fee = conn
            .prepare("PRAGMA table_info(monthly_bills)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "misc_fee" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_misc_fee {
            conn.execute_batch(
                "ALTER TABLE monthly_bills ADD COLUMN misc_fee INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE monthly_bills ADD COLUMN misc_fee_remark TEXT NOT NULL DEFAULT '';",
            )?;
            info!("[migration] misc_fee + misc_fee_remark 列添加完成");
        } else {
            info!("[migration] misc_fee 列已存在，跳过");
        }
    }

    // V2.0.7：新增收款二维码配置项
    if version_less_than(from_version, "2.0.7") {
        info!("添加收款二维码配置项...");
        // 检查是否已有该配置
        let has_qr = conn
            .query_row(
                "SELECT 1 FROM system_config WHERE config_key = '收款二维码' LIMIT 1",
                [],
                |_row| Ok(()),
            )
            .is_ok();

        if !has_qr {
            let inserted = conn.execute(
                "INSERT OR IGNORE INTO system_config (config_key, config_value, config_type, description) VALUES ('收款二维码', '', 'string', '收费通知单显示的收款二维码图片文件名')",
                [],
            )?;
            info!("[migration] 收款二维码配置项添加成功 (affected: {})", inserted);
        } else {
            info!("[migration] 收款二维码配置项已存在，跳过");
        }
    }

    // V2.0.8：新增 repair_fee 字段支持维修费
    if version_less_than(from_version, "2.0.8") {
        info!("添加 repair_fee 字段到 monthly_bills 表...");

        let has_repair_fee = conn
            .prepare("PRAGMA table_info(monthly_bills)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "repair_fee" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_repair_fee {
            conn.execute_batch(
                "ALTER TABLE monthly_bills ADD COLUMN repair_fee INTEGER NOT NULL DEFAULT 0;",
            )?;
            info!("[migration] repair_fee 列添加完成");
        } else {
            info!("[migration] repair_fee 列已存在，跳过");
        }
    }

    // V2.0.9：新增 version 字段支持乐观锁并发控制
    if version_less_than(from_version, "2.0.9") {
        info!("添加 version 字段到 rooms 表用于乐观锁...");

        let has_version = conn
            .prepare("PRAGMA table_info(rooms)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "version" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_version {
            conn.execute_batch(
                "ALTER TABLE rooms ADD COLUMN version INTEGER NOT NULL DEFAULT 0;",
            )?;
            info!("[migration] version 列添加完成");
        } else {
            info!("[migration] version 列已存在，跳过");
        }
    }

    // V2.1.0：新增 is_archived 字段支持账单归档
    if version_less_than(from_version, "2.1.0") {
        info!("添加 is_archived 字段到 monthly_bills 表用于归档支持...");

        let has_is_archived = conn
            .prepare("PRAGMA table_info(monthly_bills)")
            .ok()
            .and_then(|mut stmt| {
                let mut rows = stmt.query([]).ok()?;
                while let Some(row) = rows.next().ok()? {
                    let col_name: String = row.get("name").ok()?;
                    if col_name == "is_archived" {
                        return Some(true);
                    }
                }
                Some(false)
            })
            .unwrap_or(false);

        if !has_is_archived {
            conn.execute_batch(
                "ALTER TABLE monthly_bills ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE monthly_bills ADD COLUMN archived_at TEXT;",
            )?;
            info!("[migration] is_archived 和 archived_at 列添加完成");
        } else {
            info!("[migration] is_archived 列已存在，跳过");
        }
    }

    // V2.1.1：Phase 4 — 新增用户表、角色表、权限表
    if version_less_than(from_version, "2.1.1") {
        info!("创建用户和权限表 (users, permissions)...");

        // users 表
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
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
            )",
        )?;

        // permissions 表
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS permissions (
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
            )",
        )?;

        // 索引
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
             CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
             CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);
             CREATE INDEX IF NOT EXISTS idx_permissions_role ON permissions(role);
             CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource);",
        )?;

        // 插入默认管理员（密码：admin123）
        conn.execute(
            "INSERT OR IGNORE INTO users (username, password_hash, role, display_name, is_active)
             VALUES ('admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewKyNiiGTMwFYjOi', 'admin', '系统管理员', 1)",
            [],
        )?;

        // admin 全权限
        conn.execute_batch(
            "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
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
             ('admin', 'user', 'users', 'manage', '用户管理')",
        )?;

        // frontdesk 权限
        conn.execute_batch(
            "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
             ('frontdesk', 'room', 'rooms', 'read', '房间查看'),
             ('frontdesk', 'lease', 'leases', 'read', '合同查看'),
             ('frontdesk', 'tenant', 'tenants', 'manage', '租客管理'),
             ('frontdesk', 'bill', 'bills', 'read', '账单查看'),
             ('frontdesk', 'bill', 'bills', 'write', '账单操作'),
             ('frontdesk', 'payment', 'payments', 'manage', '缴费管理'),
             ('frontdesk', 'deposit', 'deposits', 'read', '押金查看'),
             ('frontdesk', 'report', 'reports', 'read', '报表查看'),
             ('frontdesk', 'document', 'documents', 'read', '文档查看'),
             ('frontdesk', 'excel', 'excel', 'read', 'Excel导出')",
        )?;

        // maintenance 权限
        conn.execute_batch(
            "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
             ('maintenance', 'room', 'rooms', 'read', '房间查看'),
             ('maintenance', 'room', 'rooms', 'write', '房间状态更新'),
             ('maintenance', 'lease', 'leases', 'read', '合同查看'),
             ('maintenance', 'maintenance', 'maintenance', 'manage', '维修管理'),
             ('maintenance', 'document', 'documents', 'read', '文档查看')",
        )?;

        // finance 权限
        conn.execute_batch(
            "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES
             ('finance', 'room', 'rooms', 'read', '房间查看'),
             ('finance', 'lease', 'leases', 'read', '合同查看'),
             ('finance', 'tenant', 'tenants', 'read', '租客查看'),
             ('finance', 'bill', 'bills', 'manage', '账单管理'),
             ('finance', 'payment', 'payments', 'manage', '缴费管理'),
             ('finance', 'deposit', 'deposits', 'manage', '押金管理'),
             ('finance', 'report', 'reports', 'manage', '报表管理'),
             ('finance', 'excel', 'excel', 'manage', 'Excel导入导出'),
             ('finance', 'config', 'system_config', 'read', '配置查看')",
        )?;

        info!("[migration] V2.1.1 用户权限表创建完成");
    }

    // V2.1.2：补全缺失索引
    if version_less_than(from_version, "2.1.2") {
        info!("补全缺失索引...");
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_bills_is_archived ON monthly_bills(is_archived);
             CREATE INDEX IF NOT EXISTS idx_bills_paid_date ON monthly_bills(paid_date);
             CREATE INDEX IF NOT EXISTS idx_logs_operation_type ON operation_logs(operation_type);",
        )?;
        info!("[migration] V2.1.2 索引补全完成");
    }

    Ok(())
}

/// 检查数据库是否已初始化
pub fn is_initialized(conn: &Connection) -> bool {
    let mut stmt = match conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='rooms'") {
        Ok(stmt) => stmt,
        Err(_) => return false,
    };
    stmt.exists([]).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_run_migrations() {
        let db_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(db_file.path()).unwrap();

        run_migrations(&conn).unwrap();

        assert!(is_initialized(&conn));
    }

    #[test]
    fn test_version_comparison() {
        assert!(version_less_than("2.0.1", "2.0.2"));
        assert!(version_less_than("2.0.2", "2.1.0"));
        assert!(version_less_than("1.9.9", "2.0.0"));
        assert!(version_less_than("0.0.0", "2.0.2"));
        assert!(!version_less_than("2.0.2", "2.0.2"));
        assert!(!version_less_than("2.0.3", "2.0.2"));
        assert!(version_less_than("2.0.3", "2.0.4"));
        assert!(version_less_than("2.0.4", "2.0.5"));
    }

    #[test]
    fn test_get_schema_version_new_db() {
        let db_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(db_file.path()).unwrap();

        assert_eq!(get_schema_version(&conn), "0.0.0");
    }
}
