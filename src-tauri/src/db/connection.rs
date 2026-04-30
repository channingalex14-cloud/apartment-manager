//! 数据库连接管理
//!
//! 支持线程局部连接和显式事务回滚
//! 提供 HasConnection trait，使 Service 层可接受任意实现该 trait 的上下文（生产用 AppContext，测试用 TestContext）

use rusqlite::{Connection, Transaction};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tracing::{error, info};

use crate::errors::{AppError, Result};

/// ========================
/// HasConnection trait
/// ========================

/// 连接提供者 trait
///
/// 生产环境：`AppContext` 实现
/// 测试环境：`TestContext` 实现
pub trait HasConnection {
    /// 获取数据库连接
    fn get_conn(&self) -> Result<MutexGuard<'_, Connection>>;

    /// 在连接上执行事务
    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>;
}

/// &'a T where T: HasConnection 也实现 HasConnection（便于传递引用）
impl<'a, T: HasConnection> HasConnection for &'a T {
    fn get_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        (**self).get_conn()
    }

    fn transaction<F, U>(&self, f: F) -> Result<U>
    where
        F: FnOnce(&Transaction) -> Result<U>,
    {
        (**self).transaction(f)
    }
}

/// 在任意 HasConnection 上执行事务的辅助函数
pub fn with_transaction<C: HasConnection, F, T>(ctx: &C, f: F) -> Result<T>
where
    F: FnOnce(&Transaction) -> Result<T>,
{
    let mut conn = ctx.get_conn()?;

    let tx = conn.transaction().map_err(|e| {
        error!("开启事务失败: {}", e);
        AppError::Database(e)
    })?;

    let result = f(&tx);

    match result {
        Ok(value) => {
            tx.commit().map_err(|e| {
                error!("提交事务失败: {}", e);
                AppError::Database(e)
            })?;
            Ok(value)
        }
        Err(e) => {
            if let Err(rollback_err) = tx.rollback() {
                tracing::warn!("事务回滚失败: {}", rollback_err);
            }
            error!("事务回滚: {}", e);
            Err(e)
        }
    }
}

/// ========================
/// 全局数据库路径
/// ========================

/// 全局数据库路径（通过 init_db 设置）
static DB_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 初始化数据库路径（由 Tauri 应用在启动时调用）
pub fn init_db(path: PathBuf) -> Result<()> {
    let mut db_path = DB_PATH.lock().map_err(|e| {
        AppError::Business(format!("获取数据库路径锁失败: {}", e))
    })?;
    info!("初始化数据库路径: {:?}", path);
    *db_path = Some(path);
    Ok(())
}

/// 获取数据库路径（公开接口，供备份等模块使用）
pub fn get_database_path() -> Result<PathBuf> {
    let db_path = DB_PATH.lock().map_err(|e| {
        AppError::Business(format!("获取数据库路径锁失败: {}", e))
    })?;
    db_path.clone().ok_or_else(|| {
        AppError::Business("数据库路径未初始化，请先调用 init_db".to_string())
    })
}

/// 创建新的数据库连接（线程安全，用于 spawn_blocking）
pub fn create_connection() -> Result<Connection> {
    let path = get_database_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(conn)
}

/// ========================
/// AppContext（生产环境）
/// ========================

/// 全局应用上下文单例
static APP_CONTEXT: OnceLock<AppContext> = OnceLock::new();

/// 应用上下文（持有数据库连接的单例封装）
/// 修复：使用单例模式避免重复创建数据库连接
pub struct AppContext {
    conn: OnceLock<Mutex<Connection>>,
}

impl AppContext {
    /// 获取应用上下文单例（线程安全）
    /// 修复：每次调用返回同一实例，避免重复创建数据库连接
    pub fn get_instance() -> Result<&'static Self> {
        if let Some(ctx) = APP_CONTEXT.get() {
            return Ok(ctx);
        }
        let conn = create_connection()?;
        let ctx = Self {
            conn: OnceLock::from(Mutex::new(conn)),
        };
        match APP_CONTEXT.set(ctx) {
            Ok(_) => Ok(APP_CONTEXT.get().expect("APP_CONTEXT: 刚设置成功，get 不应失败")),
            Err(_) => Ok(APP_CONTEXT.get().expect("APP_CONTEXT: set 失败说明其他线程已设置，get 不应失败")),
        }
    }

    /// 创建新的应用上下文（已废弃，请使用 get_instance）
    /// 兼容模式：如果单例已存在则返回引用，否则创建新实例
    #[deprecated(since = "2.0.5", note = "请使用 get_instance() 获取单例")]
    pub fn new() -> Result<Self> {
        // 如果单例已初始化，返回它（作为引用解引用会失败，所以我们新建一个）
        // 注意：这仍然会创建新连接，但避免了完全重复的代码
        let conn = create_connection()?;
        Ok(Self {
            conn: OnceLock::from(Mutex::new(conn)),
        })
    }

    /// 从已有连接创建应用上下文（用于测试）
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: OnceLock::from(Mutex::new(conn)),
        }
    }

    /// 获取连接引用（只读操作）
    pub fn get_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .get()
            .ok_or_else(|| AppError::Business("AppContext 未初始化".to_string()))?
            .lock()
            .map_err(|e| AppError::Business(format!("获取数据库锁失败: {}", e)))
    }

    /// 关闭数据库连接
    pub fn close(&self) {
        if let Some(conn) = self.conn.get() {
            if let Ok(_conn) = conn.lock() {
                info!("关闭数据库连接");
            }
        }
    }
}

impl Drop for AppContext {
    fn drop(&mut self) {
        self.close();
    }
}

impl HasConnection for AppContext {
    fn get_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.get_conn()
    }

    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>,
    {
        with_transaction(self, f)
    }
}

/// ========================
/// TestContext（测试环境）
/// ========================

/// 测试上下文（持有临时数据库连接）
pub struct TestContext {
    conn: Mutex<Connection>,
}

impl TestContext {
    /// 从已有连接创建测试上下文
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl HasConnection for TestContext {
    fn get_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| AppError::Business(format!("获取数据库锁失败: {}", e)))
    }

    fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>,
    {
        with_transaction(self, f)
    }
}

/// ========================
/// 全局单例访问函数
/// ========================

/// 获取全局应用上下文（懒加载单例）
/// 修复：委托给 AppContext::get_instance()，避免重复创建
pub fn get_app_context() -> Result<&'static AppContext> {
    AppContext::get_instance()
}

/// 执行数据库 VACUUM（压缩数据库文件）
///
/// 注意：VACUUM 需要独占访问，应在低峰期执行，且不能在事务内调用
pub fn vacuum_database(conn: &Connection) -> Result<()> {
    info!("开始执行 VACUUM 数据库压缩...");
    conn.execute_batch("VACUUM")
        .map_err(|e| {
            error!("VACUUM 执行失败: {}", e);
            AppError::Database(e)
        })?;
    info!("VACUUM 执行完成");
    Ok(())
}
