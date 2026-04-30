use crate::db::{create_connection, get_database_path, vacuum_database};
use crate::errors::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    pub auto_backup_enabled: bool,
    pub retention_count: i32,
    pub backup_dir: String,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_backup_enabled: false,
            retention_count: 7,
            backup_dir: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResponse {
    pub success: bool,
    pub backup_path: String,
    pub message: String,
}

pub struct BackupService;

impl BackupService {
    pub fn get_settings(conn: &rusqlite::Connection) -> Result<BackupSettings, AppError> {
        let mut settings = BackupSettings::default();

        if let Ok(mut stmt) = conn.prepare(
            "SELECT config_key, config_value FROM system_config WHERE config_key IN ('auto_backup_enabled', 'backup_retention_count', 'backup_dir')",
        ) {
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(AppError::Database)?;

            for row in rows {
                if let Ok((key, value)) = row {
                    match key.as_str() {
                        "auto_backup_enabled" => {
                            settings.auto_backup_enabled = value == "true" || value == "1"
                        }
                        "backup_retention_count" => {
                            settings.retention_count = value.parse().unwrap_or(7);
                        }
                        "backup_dir" => {
                            settings.backup_dir = value;
                        }
                        _ => {}
                    }
                }
            }
        }

        if settings.backup_dir.is_empty() {
            if let Ok(app_dir) = std::env::var("APARTMENT_DATA_DIR") {
                settings.backup_dir = PathBuf::from(app_dir)
                    .join("backups")
                    .to_string_lossy()
                    .to_string();
            } else if let Ok(db_path) = get_database_path() {
                if let Some(parent) = db_path.parent() {
                    settings.backup_dir = parent
                        .join("backups")
                        .to_string_lossy()
                        .to_string();
                }
            }
        }

        Ok(settings)
    }

    pub fn save_settings(conn: &rusqlite::Connection, settings: &BackupSettings) -> Result<(), AppError> {
        let items = vec![
            ("auto_backup_enabled", settings.auto_backup_enabled.to_string()),
            ("backup_retention_count", settings.retention_count.to_string()),
            ("backup_dir", settings.backup_dir.clone()),
        ];

        for (key, value) in items {
            conn.execute(
                "INSERT OR REPLACE INTO system_config (config_key, config_value, config_type, description)
                 VALUES (?1, ?2, 'string', '备份配置')
                 ON CONFLICT(config_key) DO UPDATE SET config_value = excluded.config_value",
                params![key, value],
            )
            .map_err(AppError::Database)?;
        }

        info!("备份设置已保存: {:?}", settings);
        Ok(())
    }

    pub fn backup(db_path: &PathBuf) -> Result<BackupResponse, AppError> {
        let db_dir = db_path.parent().ok_or_else(|| AppError::Business("无法获取数据库目录".to_string()))?;
        let backup_dir = db_dir.join("backups");

        fs::create_dir_all(&backup_dir).map_err(|e| {
            error!("创建备份目录失败: {}", e);
            AppError::Business("创建备份目录失败".to_string())
        })?;

        let conn = create_connection()?;
        vacuum_database(&conn)?;

        let now = chrono_now_string();
        let backup_filename = format!("apartment_{}.db", now);
        let backup_path = backup_dir.join(&backup_filename);

        fs::copy(db_path, &backup_path).map_err(|e| {
            error!("复制数据库文件失败: {}", e);
            AppError::Business("复制数据库文件失败".to_string())
        })?;

        let size = fs::metadata(&backup_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        info!(
            "数据库备份成功: {} -> {:?}, 大小: {} bytes",
            db_path.display(),
            backup_path,
            size
        );

        let settings = Self::get_settings_from_db_path().unwrap_or_default();
        if settings.retention_count > 0 {
            if let Err(e) = Self::cleanup_old_backups(&backup_dir, settings.retention_count) {
                error!("清理旧备份失败: {}", e);
            }
        }

        Ok(BackupResponse {
            success: true,
            backup_path: backup_path.to_string_lossy().to_string(),
            message: format!("备份成功: {} ({} bytes)", backup_filename, size),
        })
    }

    pub fn list_backups(db_path: &PathBuf) -> Result<Vec<BackupInfo>, AppError> {
        let db_dir = db_path.parent().ok_or_else(|| AppError::Business("无法获取数据库目录".to_string()))?;
        let backup_dir = db_dir.join("backups");

        if !backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups = Vec::new();

        let entries = fs::read_dir(&backup_dir).map_err(|e| {
            error!("读取备份目录失败: {}", e);
            AppError::Business("读取备份目录失败".to_string())
        })?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("读取目录项失败: {}", e);
                    continue;
                }
            };
            let path = entry.path();

            if path.extension().map(|e| e == "db").unwrap_or(false) {
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("获取文件元数据失败: {}", e);
                        continue;
                    }
                };

                let created = metadata
                    .created()
                    .ok()
                    .map(|t| chrono_datetime_string(std::time::SystemTime::from(t)))
                    .unwrap_or_else(|| "未知".to_string());

                backups.push(BackupInfo {
                    filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    size_bytes: metadata.len() as i64,
                    created_at: created,
                });
            }
        }

        backups.sort_by(|a, b| b.filename.cmp(&a.filename));

        Ok(backups)
    }

    pub fn restore(db_path: &PathBuf, backup_path: &PathBuf) -> Result<String, AppError> {
        if !backup_path.exists() {
            return Err(AppError::Business("备份文件不存在".to_string()));
        }

        if let Ok(conn) = rusqlite::Connection::open(backup_path) {
            if conn
                .query_row("SELECT COUNT(*) FROM rooms", [], |_| Ok(()))
                .is_err()
            {
                return Err(AppError::Business("备份文件不是有效的数据库文件".to_string()));
            }
        } else {
            return Err(AppError::Business("无法打开备份文件".to_string()));
        }

        let emergency_backup = db_path.with_extension("db.emergency");

        if db_path.exists() {
            fs::copy(db_path, &emergency_backup).map_err(|e| {
                error!("创建紧急备份失败: {}", e);
                AppError::Business("创建紧急备份失败".to_string())
            })?;
        }

        let db_path_clone = db_path.clone();
        let emergency_backup_clone = emergency_backup.clone();

        fs::copy(backup_path, db_path).map_err(|e| {
            error!("恢复数据库失败: {}", e);
            if emergency_backup_clone.exists() {
                if let Err(recover_err) = fs::copy(&emergency_backup_clone, &db_path_clone) {
                    error!("紧急恢复也失败！数据库可能已损坏: {}", recover_err);
                    return AppError::Business("恢复数据库失败且紧急恢复也失败".to_string());
                }
                warn!("主恢复失败，已从紧急备份恢复");
            }
            AppError::Business("恢复数据库失败".to_string())
        })?;

        info!(
            "数据库恢复成功: {:?} -> {:?}",
            backup_path,
            db_path.display()
        );

        Ok("恢复成功！紧急备份已保存".to_string())
    }

    pub fn delete_backup_file(backup_path: &PathBuf) -> Result<(), AppError> {
        if !backup_path.exists() {
            return Err(AppError::Business("备份文件不存在".to_string()));
        }

        fs::remove_file(backup_path).map_err(|e| {
            error!("删除备份文件失败: {}", e);
            AppError::Business("删除备份文件失败".to_string())
        })?;

        info!("备份文件已删除: {:?}", backup_path);
        Ok(())
    }

    fn cleanup_old_backups(backup_dir: &PathBuf, retention_count: i32) -> Result<(), AppError> {
        let entries = fs::read_dir(backup_dir).map_err(|e| {
            error!("读取备份目录失败: {}", e);
            AppError::Business("读取备份目录失败".to_string())
        })?;

        let mut db_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "db").unwrap_or(false))
            .collect();

        db_files.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));

        for (i, entry) in db_files.iter().enumerate() {
            if i >= retention_count as usize {
                if let Err(e) = fs::remove_file(entry.path()) {
                    error!("删除旧备份失败: {:?}, 错误: {}", entry.path(), e);
                } else {
                    info!("已删除旧备份: {:?}", entry.path());
                }
            }
        }

        Ok(())
    }

    fn get_settings_from_db_path() -> Result<BackupSettings, AppError> {
        let conn = create_connection()?;
        Self::get_settings(&conn)
    }
}

fn chrono_now_string() -> String {
    use std::time::SystemTime;
    chrono_datetime_string(SystemTime::now())
}

fn chrono_datetime_string(t: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Local> = t.into();
    dt.format("%Y%m%d_%H%M%S").to_string()
}
