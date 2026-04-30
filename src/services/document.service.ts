/**
 * 文档 API 服务
 */

import { callCommand } from "./api";

export interface Document {
  id: number;
  entityType: string;
  entityId: number;
  docType: string;
  originalFilename: string | null;
  storedPath: string;
  fileSize: number;
  mimeType: string | null;
  description: string | null;
  uploadedBy: string | null;
  isDeleted: boolean;
  deletedAt: string | null;
  deletedBy: string | null;
  uploadedAt: string | null;
}

export interface CreateDocumentRequest {
  entityType: string;
  entityId: number;
  docType: string;
  originalFilename?: string;
  storedPath: string;
  fileSize?: number;
  mimeType?: string;
  description?: string;
  uploadedBy?: string;
}

export interface DocumentResponse {
  success: boolean;
  documentId: number | null;
  message: string | null;
}

export interface DocumentListResponse {
  success: boolean;
  data: Document[];
  message: string | null;
}

export const documentService = {
  /** 创建文档记录 */
  async create(req: CreateDocumentRequest): Promise<DocumentResponse> {
    return callCommand<DocumentResponse>("create_document", {
      req: {
        entity_type: req.entityType,
        entity_id: req.entityId,
        doc_type: req.docType,
        original_filename: req.originalFilename,
        stored_path: req.storedPath,
        file_size: req.fileSize,
        mime_type: req.mimeType,
        description: req.description,
        uploaded_by: req.uploadedBy,
      },
    });
  },

  /** 获取文档列表 */
  async list(
    entityType?: string,
    entityId?: number,
    docType?: string
  ): Promise<DocumentListResponse> {
    return callCommand<DocumentListResponse>("list_documents", {
      entity_type: entityType,
      entity_id: entityId,
      doc_type: docType,
    });
  },

  /** 获取单个文档 */
  async get(id: number): Promise<DocumentResponse> {
    return callCommand<DocumentResponse>("get_document", { id });
  },

  /** 删除文档（软删除） */
  async delete(id: number, deletedBy?: string): Promise<DocumentResponse> {
    return callCommand<DocumentResponse>("delete_document", { id, deleted_by: deletedBy });
  },

  /** 获取实体的文档数量 */
  async getCount(entityType: string, entityId: number): Promise<number> {
    return callCommand<number>("get_document_count", { entity_type: entityType, entity_id: entityId });
  },
};

/** 文档类型选项 */
export const docTypeOptions = [
  { value: "合同扫描件", label: "合同扫描件" },
  { value: "收据", label: "收据" },
  { value: "截图", label: "截图" },
  { value: "其他", label: "其他" },
];

/** 实体类型选项 */
export const entityTypeOptions = [
  { value: "tenant", label: "租客" },
  { value: "room", label: "房间" },
  { value: "lease", label: "合同" },
  { value: "payment", label: "缴费" },
];
