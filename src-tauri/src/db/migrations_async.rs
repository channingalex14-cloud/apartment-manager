//! 异步数据库迁移（V3.0.0）
//!
//! 使用 sqlx 异步执行迁移，逻辑与 legacy migrations.rs 完全一致

use sqlx::{SqlitePool, Row};
use tracing::info;

use crate::errors::Result;

const CURRENT_VERSION: &str = "2.1.4";
const INIT_SQL: &str = include_str!("../../database/v2.0.4_schema.sql");

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

async fn get_schema_version(pool: &SqlitePool) -> String {
    let table_exists: bool = sqlx::query(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'"
    )
    .fetch_one(pool)
    .await
    .map(|row| row.get::<i64, _>(0) > 0)
    .unwrap_or(false);

    if !table_exists {
        return "0.0.0".to_string();
    }

    sqlx::query(
        "SELECT version FROM schema_version ORDER BY applied_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|row| row.get::<Option<String>, _>(0))
    .unwrap_or_else(|| "0.0.0".to_string())
}

async fn has_column(pool: &SqlitePool, table: &str, col_name: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&sql).fetch_all(pool).await;
    match rows {
        Ok(rows) => rows.iter().any(|row| {
            row.get::<String, _>("name") == col_name
        }),
        Err(_) => false,
    }
}

async fn has_config_key(pool: &SqlitePool, key: &str) -> bool {
    sqlx::query(
        "SELECT 1 FROM system_config WHERE config_key = ? LIMIT 1"
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .is_some()
}

async fn has_permission(pool: &SqlitePool, role: &str, permission: &str) -> bool {
    sqlx::query(
        "SELECT 1 FROM permissions WHERE role = ? AND permission = ? LIMIT 1"
    )
    .bind(role)
    .bind(permission)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .is_some()
}

async fn add_column_if_not_exists(
    pool: &SqlitePool,
    table: &str,
    col_def: &str,
    col_name: &str,
) -> Result<()> {
    if !has_column(pool, table, col_name).await {
        let sql = format!("ALTER TABLE {} ADD COLUMN {}", table, col_def);
        match sqlx::query(&sql).execute(pool).await {
            Ok(_) => info!("[migration] {}.{} 列添加完成", table, col_name),
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("duplicate column") {
                    info!("[migration] {}.{} 添加失败: {}", table, col_name, e);
                }
            }
        }
    } else {
        info!("[migration] {}.{} 列已存在，跳过", table, col_name);
    }
    Ok(())
}

async fn insert_config_if_not_exists(
    pool: &SqlitePool,
    key: &str,
    value: &str,
    ctype: &str,
    desc: &str,
) -> Result<()> {
    if !has_config_key(pool, key).await {
        sqlx::query(
            "INSERT OR IGNORE INTO system_config (config_key, config_value, config_type, description, is_active) VALUES (?, ?, ?, ?, 1)"
        )
        .bind(key)
        .bind(value)
        .bind(ctype)
        .bind(desc)
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("插入配置失败: {}", e)))?;
        info!("[migration] system_config: '{}' = '{}' 添加完成", key, value);
    } else {
        info!("[migration] system_config: '{}' 已存在，跳过", key);
    }
    Ok(())
}

async fn run_incremental_migrations(pool: &SqlitePool, from_version: &str) -> Result<()> {
    if from_version == "0.0.0" {
        info!("执行初始数据迁移...");
        sqlx::query(INIT_SQL)
            .execute(pool)
            .await
            .map_err(|e| crate::errors::AppError::Database(format!("初始迁移失败: {}", e)))?;
    }

    if version_less_than(from_version, "2.0.2") {
        info!("添加 room_type 字段到 rooms 表...");
        add_column_if_not_exists(pool, "rooms", "room_type TEXT DEFAULT '单间'", "room_type").await?;
    }

    if version_less_than(from_version, "2.0.4") {
        info!("创建 meter_readings 表...");
        sqlx::query(
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
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建 meter_readings 失败: {}", e)))?;
    }

    if !has_column(pool, "rooms", "room_type").await {
        info!("[migration] 发现旧数据库缺少 room_type 列，执行补充迁移...");
        add_column_if_not_exists(pool, "rooms", "room_type TEXT DEFAULT '单间'", "room_type").await?;
    }

    if version_less_than(from_version, "2.0.5") {
        info!("添加 is_deleted 列到 reminders 表...");
        add_column_if_not_exists(pool, "reminders", "is_deleted INTEGER DEFAULT 0", "is_deleted").await?;
    }

    if version_less_than(from_version, "2.0.6") {
        info!("添加 misc_fee 和 misc_fee_remark 字段到 monthly_bills 表...");
        add_column_if_not_exists(pool, "monthly_bills", "misc_fee INTEGER NOT NULL DEFAULT 0", "misc_fee").await?;
        add_column_if_not_exists(pool, "monthly_bills", "misc_fee_remark TEXT NOT NULL DEFAULT ''", "misc_fee_remark").await?;
    }

    if version_less_than(from_version, "2.0.7") {
        info!("添加收款二维码配置项...");
        insert_config_if_not_exists(
            pool, "收款二维码", "", "string", "收费通知单显示的收款二维码图片文件名",
        ).await?;
    }

    if version_less_than(from_version, "2.0.8") {
        info!("添加 repair_fee 字段到 monthly_bills 表...");
        add_column_if_not_exists(pool, "monthly_bills", "repair_fee INTEGER NOT NULL DEFAULT 0", "repair_fee").await?;
    }

    if version_less_than(from_version, "2.0.9") {
        info!("添加 version 字段到 rooms 表用于乐观锁...");
        add_column_if_not_exists(pool, "rooms", "version INTEGER NOT NULL DEFAULT 0", "version").await?;
    }

    if version_less_than(from_version, "2.1.0") {
        info!("添加 is_archived 字段到 monthly_bills 表用于归档支持...");
        add_column_if_not_exists(pool, "monthly_bills", "is_archived INTEGER NOT NULL DEFAULT 0", "is_archived").await?;
        add_column_if_not_exists(pool, "monthly_bills", "archived_at TEXT", "archived_at").await?;
    }

    if version_less_than(from_version, "2.1.1") {
        info!("创建用户和权限表 (users, permissions)...");
        sqlx::query(
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
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建 users 表失败: {}", e)))?;

        sqlx::query(
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
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建 permissions 表失败: {}", e)))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
             CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
             CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active);
             CREATE INDEX IF NOT EXISTS idx_permissions_role ON permissions(role);
             CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource);",
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建索引失败: {}", e)))?;

        sqlx::query(
            "INSERT OR IGNORE INTO users (username, password_hash, role, display_name, is_active)
             VALUES ('admin', '$2b$12$zHipk/m.xTkh.MpO9HWHNuR.FZ0Z4O2z6ixezJmlkQQKSjvJHM8cW', 'admin', '系统管理员', 1)",
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("插入默认管理员失败: {}", e)))?;

        let admin_perms = [
            ("admin", "room", "rooms", "manage", "房间管理"),
            ("admin", "lease", "leases", "manage", "合同管理"),
            ("admin", "tenant", "tenants", "manage", "租客管理"),
            ("admin", "bill", "bills", "manage", "账单管理"),
            ("admin", "payment", "payments", "manage", "缴费管理"),
            ("admin", "deposit", "deposits", "manage", "押金管理"),
            ("admin", "config", "system_config", "manage", "系统配置"),
            ("admin", "report", "reports", "read", "报表查看"),
            ("admin", "document", "documents", "manage", "文档管理"),
            ("admin", "excel", "excel", "manage", "Excel导入导出"),
            ("admin", "backup", "backup", "manage", "备份恢复"),
            ("admin", "user", "users", "manage", "用户管理"),
        ];
        for (role, perm, res, action, desc) in admin_perms {
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(role).bind(perm).bind(res).bind(action).bind(desc)
            .execute(pool).await
            .map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
        }

        let frontdesk_perms = [
            ("frontdesk", "room", "rooms", "read", "房间查看"),
            ("frontdesk", "lease", "leases", "read", "合同查看"),
            ("frontdesk", "tenant", "tenants", "manage", "租客管理"),
            ("frontdesk", "bill", "bills", "read", "账单查看"),
            ("frontdesk", "bill", "bills", "write", "账单操作"),
            ("frontdesk", "payment", "payments", "manage", "缴费管理"),
            ("frontdesk", "deposit", "deposits", "read", "押金查看"),
            ("frontdesk", "report", "reports", "read", "报表查看"),
            ("frontdesk", "document", "documents", "read", "文档查看"),
            ("frontdesk", "excel", "excel", "read", "Excel导出"),
        ];
        for (role, perm, res, action, desc) in frontdesk_perms {
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(role).bind(perm).bind(res).bind(action).bind(desc)
            .execute(pool).await
            .map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
        }

        let maintenance_perms = [
            ("maintenance", "room", "rooms", "read", "房间查看"),
            ("maintenance", "room", "rooms", "write", "房间状态更新"),
            ("maintenance", "lease", "leases", "read", "合同查看"),
            ("maintenance", "maintenance", "maintenance", "manage", "维修管理"),
            ("maintenance", "document", "documents", "read", "文档查看"),
        ];
        for (role, perm, res, action, desc) in maintenance_perms {
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(role).bind(perm).bind(res).bind(action).bind(desc)
            .execute(pool).await
            .map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
        }

        let finance_perms = [
            ("finance", "room", "rooms", "read", "房间查看"),
            ("finance", "lease", "leases", "read", "合同查看"),
            ("finance", "tenant", "tenants", "read", "租客查看"),
            ("finance", "bill", "bills", "manage", "账单管理"),
            ("finance", "payment", "payments", "manage", "缴费管理"),
            ("finance", "deposit", "deposits", "manage", "押金管理"),
            ("finance", "report", "reports", "manage", "报表管理"),
            ("finance", "excel", "excel", "manage", "Excel导入导出"),
            ("finance", "config", "system_config", "read", "配置查看"),
        ];
        for (role, perm, res, action, desc) in finance_perms {
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(role).bind(perm).bind(res).bind(action).bind(desc)
            .execute(pool).await
            .map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
        }

        info!("[migration] V2.1.1 用户权限表创建完成");
    }

    if version_less_than(from_version, "2.1.2") {
        info!("补全缺失索引...");
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_bills_is_archived ON monthly_bills(is_archived);
             CREATE INDEX IF NOT EXISTS idx_bills_paid_date ON monthly_bills(paid_date);
             CREATE INDEX IF NOT EXISTS idx_logs_operation_type ON operation_logs(operation_type);",
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建索引失败: {}", e)))?;
        info!("[migration] V2.1.2 索引补全完成");
    }

    if version_less_than(from_version, "2.1.3") {
        info!("执行 V2.1.3 租务 AI Agent 迁移...");

        add_column_if_not_exists(
            pool, "monthly_bills", "escalation_level TEXT NOT NULL DEFAULT ''", "escalation_level",
        ).await?;
        add_column_if_not_exists(
            pool, "monthly_bills", "last_reminder_at TEXT", "last_reminder_at",
        ).await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_bills_status_due ON monthly_bills(status, due_date);",
        )
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("创建索引失败: {}", e)))?;
        info!("[migration] idx_bills_status_due 索引创建完成");

        let agent_configs = [
            ("轻微逾期天数阈值", "7", "number", "逾期多少天内判定为轻微（天数）"),
            ("中度逾期天数阈值", "14", "number", "逾期多少天以上判定为中度（天数）"),
            ("严重逾期天数阈值", "15", "number", "逾期多少天以上判定为严重（天数）"),
            ("自动违约开关", "false", "boolean", "严重逾期时是否自动将房间标记为违约"),
            ("自动违约天数阈值", "30", "number", "逾期超过多少天才自动标记违约"),
            ("催租防打扰天数", "3", "number", "同一账单同一级别，多少天内不重复催租"),
            ("催租提醒人", "房东", "string", "催租提醒的署名"),
        ];
        for (key, value, ctype, desc) in agent_configs {
            insert_config_if_not_exists(pool, key, value, ctype, desc).await?;
        }

        if !has_permission(pool, "admin", "collection").await {
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES ('admin', 'collection', 'collection_agent', 'manage', '租务 AI Agent')"
            ).execute(pool).await.map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES ('frontdesk', 'collection', 'collection_agent', 'read', '租务 AI 预览')"
            ).execute(pool).await.map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
            sqlx::query(
                "INSERT OR IGNORE INTO permissions (role, permission, resource, action, description) VALUES ('finance', 'collection', 'collection_agent', 'manage', '租务 AI Agent')"
            ).execute(pool).await.map_err(|e| crate::errors::AppError::Database(format!("插入权限失败: {}", e)))?;
            info!("[migration] 催租权限添加完成");
        } else {
            info!("[migration] 催租权限已存在，跳过");
        }

        info!("[migration] V2.1.3 租务 AI Agent 迁移完成");
    }

    if version_less_than(from_version, "2.1.4") {
        info!("执行 V2.1.4 房源运营助手迁移...");

        insert_config_if_not_exists(
            pool, "合同到期提醒天数", "30", "number", "租约到期前多少天触发提醒",
        ).await?;

        add_column_if_not_exists(
            pool, "monthly_summary_cache", "new_lease_count INTEGER NOT NULL DEFAULT 0", "new_lease_count",
        ).await?;
        add_column_if_not_exists(
            pool, "monthly_summary_cache", "check_out_count INTEGER NOT NULL DEFAULT 0", "check_out_count",
        ).await?;
        add_column_if_not_exists(
            pool, "monthly_summary_cache", "vacancy_loss INTEGER NOT NULL DEFAULT 0", "vacancy_loss",
        ).await?;
        add_column_if_not_exists(
            pool, "monthly_summary_cache", "violation_rate REAL NOT NULL DEFAULT 0.0", "violation_rate",
        ).await?;

        info!("[migration] V2.1.4 房源运营助手迁移完成");
    }

    Ok(())
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let current_version = get_schema_version(pool).await;
    info!("当前数据库版本: {}, 目标版本: {}", current_version, CURRENT_VERSION);

    if current_version == CURRENT_VERSION {
        info!("数据库已是最新版本");
        return Ok(());
    }

    if !version_less_than(&current_version, CURRENT_VERSION) {
        return Err(crate::errors::AppError::Business(
            format!("数据库版本 {} 高于目标版本 {}", current_version, CURRENT_VERSION)
        ));
    }

    info!("执行数据库迁移从 {} 到 {}", current_version, CURRENT_VERSION);

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            applied_at TEXT DEFAULT (datetime('now'))
        );",
    )
    .execute(pool)
    .await
    .map_err(|e| crate::errors::AppError::Database(format!("创建 schema_version 表失败: {}", e)))?;

    run_incremental_migrations(pool, &current_version).await?;

    let needs_misc_check = current_version != "0.0.0" && version_less_than(&current_version, "2.0.6");
    if needs_misc_check {
        if !has_column(pool, "monthly_bills", "misc_fee").await {
            return Err(crate::errors::AppError::Business("misc_fee 列验证失败".to_string()));
        }
    }

    let needs_repair_check = current_version != "0.0.0" && version_less_than(&current_version, "2.0.8");
    if needs_repair_check {
        if !has_column(pool, "monthly_bills", "repair_fee").await {
            return Err(crate::errors::AppError::Business("repair_fee 列验证失败".to_string()));
        }
    }

    sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
        .bind(CURRENT_VERSION)
        .execute(pool)
        .await
        .map_err(|e| crate::errors::AppError::Database(format!("记录版本失败: {}", e)))?;

    info!("数据库迁移完成，新版本: {}", CURRENT_VERSION);
    Ok(())
}

pub async fn is_initialized(pool: &SqlitePool) -> bool {
    sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='rooms'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .is_some()
}
