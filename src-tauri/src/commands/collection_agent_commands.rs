//! 租务 AI Agent 命令

use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::models::collection_agent::{CollectionAgentReport, RunCollectionAgentRequest};
use crate::services::CollectionAgentService;
use crate::{require_admin, require_login};

/// 执行催租扫描（预览模式）— 登录即可
#[tauri::command]
pub async fn get_collection_preview(token: String) -> Result<CollectionAgentReport, String> {
    let _user = require_login!(token);

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = CollectionAgentService;

    service
        .run_agent(&ctx, &RunCollectionAgentRequest {
            dry_run: true,
            auto_escalate: false,
            auto_mark_violation: false,
            year_month: None,
        })
        .map_err(app_error_to_json_string)
}

/// 执行催租 Agent（写入数据库）— 仅管理员
#[tauri::command]
pub async fn run_collection_agent(
    token: String,
    req: RunCollectionAgentRequest,
) -> Result<CollectionAgentReport, String> {
    let _user = require_admin!(token);

    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = CollectionAgentService;

    // 不再篡改请求参数，保持用户传入的 dry_run 设置
    service
        .run_agent(&ctx, &req)
        .map_err(app_error_to_json_string)
}
