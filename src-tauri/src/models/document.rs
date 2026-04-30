//! 文档模型

use serde::{Deserialize, Serialize};

/// 文档类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Document {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub doc_type: String,
    pub original_filename: Option<String>,
    pub stored_path: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub uploaded_by: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub uploaded_at: Option<String>,
}

/// 创建文档请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateDocumentRequest {
    pub entity_type: String,
    pub entity_id: i64,
    pub doc_type: String,
    pub original_filename: Option<String>,
    pub stored_path: String,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub uploaded_by: Option<String>,
}

/// 文档响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DocumentResponse {
    pub success: bool,
    pub document_id: Option<i64>,
    pub message: Option<String>,
}

/// 文档列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DocumentListResponse {
    pub success: bool,
    pub data: Vec<Document>,
    pub message: Option<String>,
}
