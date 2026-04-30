//! 数据库层
//!
//! 连接管理、事务封装、查询模板
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: rusqlite 同步实现
//! - new-db: sqlx 异步实现（Phase 1）

#[cfg(feature = "legacy-db")]
pub mod connection;
#[cfg(feature = "new-db")]
pub mod connection_async;

#[cfg(feature = "legacy-db")]
pub mod migrations;
#[cfg(feature = "new-db")]
pub mod migrations_async;

#[cfg(feature = "legacy-db")]
pub mod queries;
#[cfg(feature = "new-db")]
pub mod queries_async;

#[cfg(test)]
#[cfg(feature = "legacy-db")]
pub mod test_helpers;

#[cfg(feature = "legacy-db")]
pub use connection::*;
#[cfg(feature = "new-db")]
pub use connection_async::*;

#[cfg(feature = "legacy-db")]
pub use migrations::*;
#[cfg(feature = "new-db")]
pub use migrations_async::*;

#[cfg(feature = "legacy-db")]
pub use queries::*;
#[cfg(feature = "new-db")]
pub use queries_async::*;
