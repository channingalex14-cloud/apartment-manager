<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { open } from "@tauri-apps/plugin-dialog";
import { documentService } from "@/services/document.service";
import { docTypeOptions, entityTypeOptions } from "@/services/document.service";
import type { Document, CreateDocumentRequest } from "@/services/document.service";

const loading = ref(false);
const documents = ref<Document[]>([]);

const filterEntityType = ref<string | undefined>(undefined);
const filterDocType = ref<string | undefined>(undefined);

const createDialogVisible = ref(false);
const createForm = ref<CreateDocumentRequest>({
  entityType: "",
  entityId: 0,
  docType: "",
  originalFilename: "",
  storedPath: "",
  fileSize: undefined,
  mimeType: "",
  description: "",
  uploadedBy: "",
});

const submitting = ref(false);

onMounted(async () => {
  await fetchDocuments();
});

async function fetchDocuments() {
  loading.value = true;
  try {
    const resp = await documentService.list(filterEntityType.value, undefined, filterDocType.value);
    if (resp.success) {
      documents.value = resp.data || [];
    } else {
      ElMessage.error(resp.message || "获取文档失败");
    }
  } catch {
    ElMessage.error("获取文档失败");
  } finally {
    loading.value = false;
  }
}

async function onFilterChange() {
  await fetchDocuments();
}

function openCreateDialog() {
  createForm.value = {
    entityType: "",
    entityId: 0,
    docType: "",
    originalFilename: "",
    storedPath: "",
    fileSize: undefined,
    mimeType: "",
    description: "",
    uploadedBy: "",
  };
  createDialogVisible.value = true;
}

async function handleSelectFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "所有文件", extensions: ["*"] },
        { name: "PDF", extensions: ["pdf"] },
        { name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "bmp"] },
        { name: "Word", extensions: ["doc", "docx"] },
        { name: "Excel", extensions: ["xls", "xlsx"] },
      ],
    });
    if (selected) {
      const path = selected as string;
      createForm.value.storedPath = path;
      const filename = path.split(/[/\\]/).pop() || path;
      createForm.value.originalFilename = filename;
      const ext = filename.split(".").pop()?.toLowerCase() || "";
      const mimeMap: Record<string, string> = {
        pdf: "application/pdf",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        png: "image/png",
        gif: "image/gif",
        bmp: "image/bmp",
        doc: "application/msword",
        docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        xls: "application/vnd.ms-excel",
        xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
      };
      createForm.value.mimeType = mimeMap[ext] || "application/octet-stream";
    }
  } catch {
    ElMessage.error("选择文件失败");
  }
}

async function submitCreate() {
  if (!createForm.value.entityType || !createForm.value.entityId || !createForm.value.docType || !createForm.value.storedPath) {
    ElMessage.warning("请填写必填项（实体类型、关联ID、文档类型、文件）");
    return;
  }
  submitting.value = true;
  try {
    const resp = await documentService.create(createForm.value);
    if (resp.success) {
      ElMessage.success("创建成功");
      createDialogVisible.value = false;
      await fetchDocuments();
    } else {
      ElMessage.error(resp.message || "创建失败");
    }
  } catch {
    ElMessage.error("创建失败");
  } finally {
    submitting.value = false;
  }
}

async function deleteDoc(id: number) {
  try {
    await ElMessageBox.confirm("确定删除此文档记录？", "确认删除", { type: "warning" });
    const resp = await documentService.delete(id);
    if (resp.success) {
      ElMessage.success("删除成功");
      await fetchDocuments();
    } else {
      ElMessage.error(resp.message || "删除失败");
    }
  } catch {
    // cancelled
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "-";
  return dateStr.slice(0, 16);
}

function formatFileSize(bytes: number | null | undefined): string {
  if (!bytes) return "-";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

function getDocTypeLabel(value: string): string {
  const opt = docTypeOptions.find((o) => o.value === value);
  return opt?.label || value;
}

function getEntityTypeLabel(value: string): string {
  const opt = entityTypeOptions.find((o) => o.value === value);
  return opt?.label || value;
}

function getDocTypeTagType(type: string): "primary" | "success" | "warning" | "info" | "danger" {
  const map: Record<string, "primary" | "success" | "warning" | "info" | "danger"> = {
    合同扫描件: "primary",
    收据: "success",
    截图: "warning",
    其他: "info",
  };
  return map[type] || "info";
}
</script>

<template>
  <div class="documents">
    <!-- 筛选栏 -->
    <el-card class="tool-card">
      <div class="toolbar">
        <div class="filters">
          <el-select
            v-model="filterEntityType"
            placeholder="实体类型"
            clearable
            @change="onFilterChange"
            style="width: 140px; margin-right: 8px"
          >
            <el-option
              v-for="opt in entityTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <el-select
            v-model="filterDocType"
            placeholder="文档类型"
            clearable
            @change="onFilterChange"
            style="width: 140px"
          >
            <el-option
              v-for="opt in docTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </div>
        <el-button type="primary" @click="openCreateDialog">新建文档</el-button>
      </div>
    </el-card>

    <!-- 文档列表 -->
    <el-card class="doc-card" v-loading="loading">
      <template #header>
        <div class="card-header">
          <span>文档列表</span>
        </div>
      </template>

      <el-table :data="documents" stripe>
        <el-table-column prop="docType" label="类型" width="120">
          <template #default="{ row }">
            <el-tag :type="getDocTypeTagType(row.docType)" size="small">
              {{ getDocTypeLabel(row.docType) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="entityType" label="关联类型" width="100">
          <template #default="{ row }">
            {{ getEntityTypeLabel(row.entityType) }}
          </template>
        </el-table-column>
        <el-table-column prop="entityId" label="关联ID" width="100" />
        <el-table-column prop="originalFilename" label="文件名" min-width="200">
          <template #default="{ row }">
            {{ row.originalFilename || row.storedPath.split("/").pop() || "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="fileSize" label="大小" width="100">
          <template #default="{ row }">
            {{ formatFileSize(row.fileSize) }}
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="150">
          <template #default="{ row }">
            {{ row.description || "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="uploadedBy" label="上传人" width="100">
          <template #default="{ row }">
            {{ row.uploadedBy || "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="uploadedAt" label="上传时间" width="160">
          <template #default="{ row }">
            {{ formatDate(row.uploadedAt) }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80">
          <template #default="{ row }">
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteDoc(row.id)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="documents.length === 0 && !loading" class="empty-tip">
        <el-empty description="暂无文档数据" />
      </div>
    </el-card>

    <!-- 创建对话框 -->
    <el-dialog v-model="createDialogVisible" title="新建文档记录" width="500px">
      <el-form :model="createForm" label-width="100px">
        <el-form-item label="实体类型" required>
          <el-select v-model="createForm.entityType" placeholder="选择类型">
            <el-option
              v-for="opt in entityTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="关联ID" required>
          <el-input-number
            v-model="createForm.entityId"
            :min="0"
            placeholder="关联的业务ID"
          />
        </el-form-item>
        <el-form-item label="文档类型" required>
          <el-select v-model="createForm.docType" placeholder="选择类型">
            <el-option
              v-for="opt in docTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="选择文件" required>
          <el-input v-model="createForm.storedPath" placeholder="点击右侧按钮选择文件" readonly style="width: 280px" />
          <el-button type="primary" style="margin-left: 8px" @click="handleSelectFile">选择文件</el-button>
        </el-form-item>
        <el-form-item v-if="createForm.originalFilename" label="文件名">
          <span>{{ createForm.originalFilename }}</span>
        </el-form-item>
        <el-form-item v-if="createForm.mimeType" label="文件类型">
          <el-tag size="small">{{ createForm.mimeType }}</el-tag>
        </el-form-item>
        <el-form-item label="描述">
          <el-input
            v-model="createForm.description"
            type="textarea"
            :rows="2"
            placeholder="文档描述"
          />
        </el-form-item>
        <el-form-item label="上传人">
          <el-input v-model="createForm.uploadedBy" placeholder="上传人姓名" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">
          创建
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.documents {
  .tool-card {
    margin-top: 16px;
  }

  .doc-card {
    margin-top: 16px;

    .card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .filters {
    display: flex;
    align-items: center;
  }

  .empty-tip {
    margin-top: 20px;
  }
}
</style>
