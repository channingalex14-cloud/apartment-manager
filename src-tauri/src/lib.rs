//! 新逸公寓管理系统 V2.0.5
//!
//! Rust 后端入口
//!
//! V3.0.0 条件编译隔离：
//! - legacy-db: rusqlite 同步实现（默认）
//! - new-db: sqlx 异步实现（Phase 1）

pub mod commands;
pub mod db;
pub mod errors;
pub mod interceptors;
pub mod models;
pub mod services;
pub mod utils;

#[cfg(feature = "legacy-db")]
pub mod auth;

#[cfg(feature = "new-db")]
pub mod auth_async;

// Phase 0 阻塞门禁验证原型（new-db feature）
#[cfg(feature = "new-db")]
pub mod phase0_validation;

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static LOG_GUARD: LazyLock<Mutex<Option<WorkerGuard>>> = LazyLock::new(|| Mutex::new(None));

fn init_logging(app_log_dir: &Path) {
    std::fs::create_dir_all(app_log_dir).ok();

    let file_appender = RollingFileAppender::new(
        Rotation::HOURLY,
        app_log_dir,
        "apartment.log",
    );
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    *LOG_GUARD.lock().unwrap() = Some(guard);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
        )
        .with(filter)
        .init();

    info!("新逸公寓管理系统 V2.0.5 启动中...");
}

/// 初始化应用（使用默认日志目录）
pub fn init() {
    // 日志目录改为应用数据目录下，避免在源码目录创建日志
    let app_data_dir = dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    let log_dir = app_data_dir.join("com.xinyi.apartment-manager").join("logs");
    init_logging(&log_dir);
    info!("日志系统初始化完成");
}

/// 重新初始化日志系统（带正确的应用数据目录）
pub fn init_logging_with_dir(app_log_dir: PathBuf) {
    init_logging(&app_log_dir);
}
