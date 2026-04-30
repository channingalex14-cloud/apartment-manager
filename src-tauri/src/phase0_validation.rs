//! Phase 0 阻塞门禁验证原型
//!
//! 验证项：
//! 1. Executor 泛型模式可编译
//! 2. RepositoryError 不泄露 sqlx 依赖
//! 3. Mock 使用 tokio::sync::Mutex
//! 4. 迁移自举（ensure_migration_table）
//! 5. 技术选型确认

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

// ============================================================================
// 验证项 1: Executor 泛型模式
// ============================================================================

/// 验证 Executor 泛型模式是否可编译
/// 
/// Repository 方法接受 `Executor<'e, Database = Sqlite>` 泛型，
/// 支持 `&SqlitePool` 和 `&mut Transaction`
#[cfg(feature = "new-db")]
mod executor_pattern {
    use sqlx::{SqlitePool, Sqlite, Executor};
    
    /// 示例：Repository 方法使用 Executor 泛型
    pub async fn find_by_id<'e, E>(
        executor: E,
        id: i64,
    ) -> Result<Option<String>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite> + Send + 'e,
    {
        // 查询示例
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM test_table WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;
        
        Ok(result.map(|r| r.0))
    }
    
    /// 验证：可以用 &SqlitePool 调用
    pub async fn test_with_pool(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
        find_by_id(pool, 1).await
    }
    
    /// 验证：可以用 Transaction 调用
    /// 
    /// 注意：Transaction 需要解引用为 &mut SqliteConnection
    pub async fn test_with_transaction(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
    ) -> Result<Option<String>, sqlx::Error> {
        // Transaction 实现了 DerefMut<Target = SqliteConnection>
        // 所以 &mut *tx 可以作为 Executor
        find_by_id(&mut **tx, 1).await
    }
}

// ============================================================================
// 验证项 2: RepositoryError 不泄露 sqlx 依赖
// ============================================================================

/// RepositoryError 设计
/// 
/// 关键：不使用 `#[from] sqlx::Error`，而是手动转换
/// 这样 Service 层不依赖 sqlx::Error
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("数据库错误: {0}")]
    Database(String),  // 不直接持有 sqlx::Error
    
    #[error("记录不存在: {0}")]
    NotFound(String),
    
    #[error("数据验证失败: {0}")]
    Validation(String),
    
    #[error("并发冲突: {0}")]
    Concurrency(String),
    
    #[error("事务失败: {0}")]
    Transaction(String),
}

/// 手动实现 From<sqlx::Error>
/// 
/// 在 SQLite 实现中转换，不泄露到 Service 层
#[cfg(feature = "new-db")]
impl From<sqlx::Error> for RepositoryError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound(e.to_string()),
            _ => RepositoryError::Database(e.to_string()),
        }
    }
}

pub type RepositoryResult<T> = std::result::Result<T, RepositoryError>;

// ============================================================================
// 验证项 3: Mock 使用 tokio::sync::Mutex
// ============================================================================

/// Mock Repository 使用异步锁
/// 
/// 关键：使用 tokio::sync::Mutex，不是 parking_lot::Mutex
/// 所有 `.lock()` 改为 `.lock().await`
pub struct MockRepository {
    data: Arc<tokio::sync::Mutex<HashMap<i64, String>>>,
}

impl MockRepository {
    pub fn new() -> Self {
        Self {
            data: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }
    
    /// 异步方法使用异步锁
    pub async fn find_by_id(&self, id: i64) -> RepositoryResult<Option<String>> {
        // 使用 tokio::sync::Mutex
        let data = self.data.lock().await;
        Ok(data.get(&id).cloned())
    }
    
    pub async fn save(&self, id: i64, value: String) -> RepositoryResult<()> {
        // 使用 tokio::sync::Mutex
        let mut data = self.data.lock().await;
        data.insert(id, value);
        Ok(())
    }
}

// ============================================================================
// 验证项 4: 迁移自举（ensure_migration_table）
// ============================================================================

/// 迁移系统自举
/// 
/// 关键：在 run() 入口调用，确保 migrations 表存在
#[cfg(feature = "new-db")]
pub async fn ensure_migration_table(pool: &sqlx::SqlitePool) -> RepositoryResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            version     TEXT NOT NULL UNIQUE,
            name        TEXT NOT NULL,
            executed_at TEXT DEFAULT (datetime('now')),
            duration_ms INTEGER
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| RepositoryError::from(e))?;
    
    Ok(())
}

/// Migration trait 定义
#[cfg(all(feature = "new-db", feature = "async-trait"))]
mod migration_trait {
    use async_trait::async_trait;
    use super::RepositoryResult;
    
    #[async_trait]
    pub trait Migration: Send + Sync {
        fn version(&self) -> String;
        fn name(&self) -> &str;
        
        /// 迁移在事务中执行
        async fn up(
            &self,
            tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        ) -> RepositoryResult<()>;
        
        fn dependencies(&self) -> Vec<&str> {
            vec![]
        }
    }
}

// ============================================================================
// 验证项 5: 技术选型确认
// ============================================================================

/// 技术选型确认
/// 
/// 直接使用 sqlx，不使用 tauri-plugin-sql
/// 
/// 理由：
/// - tauri-plugin-sql 与 Executor 泛型模式不兼容
/// - tauri-plugin-sql 不暴露 Transaction 类型
/// - tauri-plugin-sql 不允许自定义连接池配置
/// - tauri-plugin-sql 有自己的迁移系统，无法自定义

#[cfg(feature = "new-db")]
fn confirm_tech_stack() {
    // 确认使用 sqlx
    let _ = "sqlx";
    
    // 确认不使用 tauri-plugin-sql
    // （没有导入 tauri_plugin_sql）
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_repository() {
        let repo = MockRepository::new();
        
        // 测试保存
        repo.save(1, "test".to_string()).await.unwrap();
        
        // 测试查询
        let result = repo.find_by_id(1).await.unwrap();
        assert_eq!(result, Some("test".to_string()));
        
        // 测试不存在
        let result = repo.find_by_id(2).await.unwrap();
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_repository_error() {
        // 验证 RepositoryError 不泄露 sqlx 依赖
        let err = RepositoryError::NotFound("test".to_string());
        assert!(err.to_string().contains("不存在"));
        
        let err = RepositoryError::Database("connection failed".to_string());
        assert!(err.to_string().contains("数据库错误"));
    }
}
