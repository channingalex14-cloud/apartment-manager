//! 业务逻辑层
//!
//! 核心业务逻辑：状态机、押金台账、账单生成
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: rusqlite 同步实现（默认）
//! - new-db: sqlx 异步实现（Phase 3）

#[cfg(feature = "legacy-db")]
pub mod lease_service;
#[cfg(feature = "legacy-db")]
pub mod bill_service;
#[cfg(feature = "legacy-db")]
pub mod payment_service;
#[cfg(feature = "legacy-db")]
pub mod deposit_service;
#[cfg(feature = "legacy-db")]
pub mod meter_reading_service;
#[cfg(feature = "legacy-db")]
pub mod report_service;
#[cfg(feature = "legacy-db")]
pub mod reminder_service;
#[cfg(feature = "legacy-db")]
pub mod document_service;
#[cfg(feature = "legacy-db")]
pub mod export_service;
#[cfg(feature = "legacy-db")]
pub mod diagnostic_service;
#[cfg(feature = "legacy-db")]
pub mod import_service;
#[cfg(feature = "legacy-db")]
pub mod backup_service;
#[cfg(feature = "legacy-db")]
pub mod room_service;

#[cfg(feature = "legacy-db")]
pub use lease_service::*;
#[cfg(feature = "legacy-db")]
pub use bill_service::*;
#[cfg(feature = "legacy-db")]
pub use payment_service::*;
#[cfg(feature = "legacy-db")]
pub use deposit_service::*;
#[cfg(feature = "legacy-db")]
pub use meter_reading_service::*;
#[cfg(feature = "legacy-db")]
pub use report_service::*;
#[cfg(feature = "legacy-db")]
pub use reminder_service::*;
#[cfg(feature = "legacy-db")]
pub use document_service::*;
#[cfg(feature = "legacy-db")]
pub use export_service::*;
#[cfg(feature = "legacy-db")]
pub use diagnostic_service::*;
#[cfg(feature = "legacy-db")]
pub use import_service::*;
#[cfg(feature = "legacy-db")]
pub use backup_service::*;
#[cfg(feature = "legacy-db")]
pub use room_service::*;

// ============================================================================
// new-db: 异步 Service（Phase 3）
// TODO: room_service_async 有 lifetime 问题，待 GLM 协助解决
// ============================================================================

#[cfg(feature = "new-db")]
pub mod lease_service_async;
#[cfg(feature = "new-db")]
pub mod bill_service_async;
#[cfg(feature = "new-db")]
pub mod payment_service_async;
#[cfg(feature = "new-db")]
pub mod deposit_service_async;
#[cfg(feature = "new-db")]
pub mod meter_reading_service_async;
#[cfg(feature = "new-db")]
pub mod reminder_service_async;
#[cfg(feature = "new-db")]
pub mod document_service_async;
#[cfg(feature = "new-db")]
pub mod report_service_async;
#[cfg(feature = "new-db")]
pub mod export_service_async;
#[cfg(feature = "new-db")]
pub mod import_service_async;
#[cfg(feature = "new-db")]
pub mod backup_service_async;
#[cfg(feature = "new-db")]
pub mod diagnostic_service_async;
#[cfg(feature = "new-db")]
pub mod collection_agent_service_async;

#[cfg(feature = "new-db")]
pub use lease_service_async::*;
#[cfg(feature = "new-db")]
pub use bill_service_async::*;
#[cfg(feature = "new-db")]
pub use payment_service_async::*;
#[cfg(feature = "new-db")]
pub use deposit_service_async::*;
#[cfg(feature = "new-db")]
pub use meter_reading_service_async::*;
#[cfg(feature = "new-db")]
pub use reminder_service_async::*;
#[cfg(feature = "new-db")]
pub use document_service_async::*;
#[cfg(feature = "new-db")]
pub use report_service_async::*;
#[cfg(feature = "new-db")]
pub use export_service_async::*;
#[cfg(feature = "new-db")]
pub use import_service_async::*;
#[cfg(feature = "new-db")]
pub use backup_service_async::*;
#[cfg(feature = "new-db")]
pub use diagnostic_service_async::*;
#[cfg(feature = "new-db")]
pub use collection_agent_service_async::*;
