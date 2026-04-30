//! 数据模型
//!
//! 对应数据库表结构
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: rusqlite 同步实现（默认）
//! - new-db: sqlx 异步实现（Phase 1）

#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod room;

#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod tenant;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod lease;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod bill;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod payment;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod deposit;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod config;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod meter_reading;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod report;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod reminder;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod document;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub mod collection_agent;

pub use room::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use tenant::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use lease::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use bill::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use payment::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use deposit::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use config::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use meter_reading::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use report::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use reminder::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use document::*;
#[cfg(any(feature = "legacy-db", feature = "new-db"))]
pub use collection_agent::*;
