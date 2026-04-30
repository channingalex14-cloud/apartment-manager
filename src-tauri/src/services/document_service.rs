//! 文档服务
//!
//! 文档存储管理

use crate::db::connection::HasConnection;
use crate::errors::{AppError, Result};
use crate::models::document::{
    CreateDocumentRequest, Document, DocumentListResponse, DocumentResponse,
};
use rusqlite::params;
use tracing::info;

/// 文档服务
pub struct DocumentService;

impl DocumentService {
    /// 创建文档记录
    pub fn create_document<C: HasConnection>(
        &self,
        ctx: &C,
        req: &CreateDocumentRequest,
    ) -> Result<DocumentResponse> {
        info!(
            "创建文档: type={}, entity={}/{}",
            req.doc_type, req.entity_type, req.entity_id
        );

        let id = ctx.transaction(|tx| {
            tx.execute(
                r#"
                INSERT INTO documents
                (entity_type, entity_id, doc_type, original_filename, stored_path,
                 file_size, mime_type, description, uploaded_by, is_deleted)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
                "#,
                params![
                    req.entity_type,
                    req.entity_id,
                    req.doc_type,
                    req.original_filename,
                    req.stored_path,
                    req.file_size.unwrap_or(0),
                    req.mime_type,
                    req.description,
                    req.uploaded_by,
                ],
            )?;
            Ok(tx.last_insert_rowid())
        })?;

        info!("创建文档成功: id={}", id);
        Ok(DocumentResponse {
            success: true,
            document_id: Some(id),
            message: None,
        })
    }

    /// 获取文档列表
    pub fn list_documents<C: HasConnection>(
        &self,
        ctx: &C,
        entity_type: Option<String>,
        entity_id: Option<i64>,
        doc_type: Option<String>,
    ) -> Result<DocumentListResponse> {
        ctx.transaction(|tx| {
            let mut sql = "SELECT * FROM documents WHERE is_deleted = 0".to_string();
            let mut conditions = Vec::new();

            if entity_type.is_some() {
                conditions.push("entity_type = ?");
            }
            if entity_id.is_some() {
                conditions.push("entity_id = ?");
            }
            if doc_type.is_some() {
                conditions.push("doc_type = ?");
            }

            if !conditions.is_empty() {
                sql.push_str(" AND ");
                sql.push_str(&conditions.join(" AND "));
            }
            sql.push_str(" ORDER BY uploaded_at DESC");

            let mut stmt = tx.prepare(&sql)?;

            let rows = match (entity_type, entity_id, doc_type) {
                (Some(et), Some(eid), Some(dt)) => {
                    stmt.query_map(params![et, eid, dt], map_document)?
                }
                (Some(et), Some(eid), None) => stmt.query_map(params![et, eid], map_document)?,
                (Some(et), None, Some(dt)) => stmt.query_map(params![et, dt], map_document)?,
                (Some(et), None, None) => stmt.query_map(params![et], map_document)?,
                (None, Some(eid), Some(dt)) => stmt.query_map(params![eid, dt], map_document)?,
                (None, Some(eid), None) => stmt.query_map(params![eid], map_document)?,
                (None, None, Some(dt)) => stmt.query_map(params![dt], map_document)?,
                (None, None, None) => stmt.query_map([], map_document)?,
            };

            let mut documents = Vec::new();
            for row in rows {
                documents.push(row?);
            }

            Ok(DocumentListResponse {
                success: true,
                data: documents,
                message: None,
            })
        })
    }

    /// 获取单个文档
    pub fn get_document<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
    ) -> Result<DocumentResponse> {
        ctx.transaction(|tx| {
            let mut stmt = tx.prepare("SELECT * FROM documents WHERE id = ? AND is_deleted = 0")?;
            let result = stmt.query_row([id], map_document).ok();

            match result {
                Some(doc) => Ok(DocumentResponse {
                    success: true,
                    document_id: Some(doc.id),
                    message: None,
                }),
                None => Ok(DocumentResponse {
                    success: false,
                    document_id: None,
                    message: Some(format!("文档不存在: id={}", id)),
                }),
            }
        })
    }

    /// 删除文档（软删除）
    pub fn delete_document<C: HasConnection>(
        &self,
        ctx: &C,
        id: i64,
        deleted_by: Option<&str>,
    ) -> Result<DocumentResponse> {
        ctx.transaction(|tx| {
            let affected = tx.execute(
                "UPDATE documents SET is_deleted = 1, deleted_at = datetime('now'), deleted_by = ? WHERE id = ? AND is_deleted = 0",
                params![deleted_by, id],
            ).map_err(|e| AppError::Database(e))?;

            if affected > 0 {
                Ok(DocumentResponse {
                    success: true,
                    document_id: Some(id),
                    message: None,
                })
            } else {
                Ok(DocumentResponse {
                    success: false,
                    document_id: None,
                    message: Some(format!("文档不存在: id={}", id)),
                })
            }
        })
    }

    /// 获取实体的文档数量
    pub fn get_document_count<C: HasConnection>(
        &self,
        ctx: &C,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<i64> {
        let conn = ctx.get_conn()?;

        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM documents WHERE entity_type = ? AND entity_id = ? AND is_deleted = 0",
        )?;

        let count: i64 = stmt.query_row(params![entity_type, entity_id], |row| row.get(0))?;

        Ok(count)
    }
}

/// 映射数据库行为 Document
fn map_document(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get("id")?,
        entity_type: row.get("entity_type")?,
        entity_id: row.get("entity_id")?,
        doc_type: row.get("doc_type")?,
        original_filename: row.get("original_filename")?,
        stored_path: row.get("stored_path")?,
        file_size: row.get("file_size")?,
        mime_type: row.get("mime_type")?,
        description: row.get("description")?,
        uploaded_by: row.get("uploaded_by")?,
        is_deleted: row.get::<_, i64>("is_deleted")? != 0,
        deleted_at: row.get("deleted_at")?,
        deleted_by: row.get("deleted_by")?,
        uploaded_at: row.get("uploaded_at")?,
    })
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_doc_type_list() {
        let types = vec![
            "合同扫描件",
            "收据",
            "截图",
            "其他",
        ];
        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_entity_type_list() {
        let types = vec![
            "tenant",
            "room",
            "lease",
            "payment",
        ];
        assert_eq!(types.len(), 4);
    }
}
