//! 文档命令

use crate::db::get_app_context;
use crate::errors::app_error_to_json_string;
use crate::models::document::{
    CreateDocumentRequest, DocumentListResponse, DocumentResponse,
};
use crate::{require_admin, require_login};
use crate::services::DocumentService;

/// 创建文档记录
#[tauri::command]
pub async fn create_document(token: String, req: CreateDocumentRequest) -> Result<DocumentResponse, String> {
    let _user = require_login!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = DocumentService;
    service
        .create_document(&ctx, &req)
        .map_err(app_error_to_json_string)
}

/// 获取文档列表
#[tauri::command]
pub async fn list_documents(
    entity_type: Option<String>,
    entity_id: Option<i64>,
    doc_type: Option<String>,
) -> Result<DocumentListResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = DocumentService;
    service
        .list_documents(&ctx, entity_type, entity_id, doc_type)
        .map_err(app_error_to_json_string)
}

/// 获取单个文档
#[tauri::command]
pub async fn get_document(id: i64) -> Result<DocumentResponse, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = DocumentService;
    service.get_document(&ctx, id).map_err(app_error_to_json_string)
}

/// 删除文档（软删除）
#[tauri::command]
pub async fn delete_document(
    token: String,
    id: i64,
    deleted_by: Option<String>,
) -> Result<DocumentResponse, String> {
    let _user = require_admin!(token);
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = DocumentService;
    service
        .delete_document(&ctx, id, deleted_by.as_deref())
        .map_err(app_error_to_json_string)
}

/// 获取实体的文档数量
#[tauri::command]
pub async fn get_document_count(
    entity_type: String,
    entity_id: i64,
) -> Result<i64, String> {
    let ctx = get_app_context().map_err(app_error_to_json_string)?;
    let service = DocumentService;
    service
        .get_document_count(&ctx, &entity_type, entity_id)
        .map_err(app_error_to_json_string)
}
