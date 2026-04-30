//! 异步数据库连接管理（V3.0.0）
//!
//! 使用 sqlx 异步连接池，替代 rusqlite 同步单连接

use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::info;

use crate::errors::{AppError, Result};

static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

static DB_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub async fn init_db_async(path: PathBuf) -> Result<()> {
    info!("初始化异步数据库连接池: {:?}", path);

    {
        let mut db_path = DB_PATH.lock().map_err(|e| {
            AppError::Business(format!("获取数据库路径锁失败: {}", e))
        })?;
        *db_path = Some(path.clone());
    }

    let connection_string = format!("sqlite:{}?mode=rwc", path.display());
    let pool = SqlitePool::connect(&connection_string).await.map_err(|e| {
        AppError::Database(format!("连接池创建失败: {}", e))
    })?;

    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(format!("设置 PRAGMA foreign_keys 失败: {}", e)))?;

    sqlx::query("PRAGMA busy_timeout = 5000;")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Database(format!("设置 PRAGMA busy_timeout 失败: {}", e)))?;

    DB_POOL.set(pool).map_err(|_| AppError::Business("连接池已初始化".to_string()))?;

    info!("异步数据库连接池初始化完成");
    Ok(())
}

pub fn get_pool() -> Result<&'static SqlitePool> {
    DB_POOL.get().ok_or_else(|| {
        AppError::Business("数据库连接池未初始化，请先调用 init_db_async".to_string())
    })
}

pub fn get_database_path() -> Result<PathBuf> {
    let db_path = DB_PATH.lock().map_err(|e| {
        AppError::Business(format!("获取数据库路径锁失败: {}", e))
    })?;
    db_path.clone().ok_or_else(|| {
        AppError::Business("数据库路径未初始化，请先调用 init_db_async".to_string())
    })
}

pub async fn close_db() {
    if let Some(pool) = DB_POOL.get() {
        pool.close().await;
        info!("关闭异步数据库连接池");
    }
}

pub struct AppContextAsync {
    pool: SqlitePool,
}

impl AppContextAsync {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<T>,
    {
        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::Database(format!("开启事务失败: {}", e))
        })?;

        let result = f(&mut tx);

        match result {
            Ok(value) => {
                tx.commit().await.map_err(|e| {
                    AppError::Database(format!("提交事务失败: {}", e))
                })?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    tracing::warn!("事务回滚失败: {}", rollback_err);
                }
                Err(e)
            }
        }
    }

    pub async fn transaction_with_async<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::Database(format!("开启事务失败: {}", e))
        })?;

        let result = f(&mut tx).await;

        match result {
            Ok(value) => {
                tx.commit().await.map_err(|e| {
                    AppError::Database(format!("提交事务失败: {}", e))
                })?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    tracing::warn!("事务回滚失败: {}", rollback_err);
                }
                Err(e)
            }
        }
    }
}

pub async fn vacuum_database(pool: &SqlitePool) -> Result<()> {
    info!("开始执行 VACUUM 数据库压缩...");
    sqlx::query("VACUUM")
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("VACUUM 执行失败: {}", e);
            AppError::Database(format!("VACUUM 执行失败: {}", e))
        })?;
    info!("VACUUM 执行完成");
    Ok(())
}
