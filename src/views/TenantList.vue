<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { tenantService, type CreateTenantRequest, type UpdateTenantRequest } from "@/services/tenant.service";
import { useUIStore } from "@/stores/ui";
import type { TenantResponse } from "@/types/tenant";
import { z } from "zod";

const uiStore = useUIStore();

const tenants = ref<TenantResponse[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const isEditing = ref(false);
const editingId = ref(0);

const formData = ref({
  name: "",
  phone: "",
  phone2: "",
  emergency_contact: "",
  emergency_phone: "",
});

const tenantSchema = z.object({
  name: z.string().min(1, "姓名不能为空").max(50),
  phone: z.string().regex(/^1[3-9]\d{9}$/, "请输入有效的11位手机号"),
  phone2: z.string().regex(/^1[3-9]\d{9}$/, "请输入有效的11位手机号").optional().or(z.literal("")),
  emergency_contact: z.string().max(50).optional().or(z.literal("")),
  emergency_phone: z.string().regex(/^1[3-9]\d{9}$/, "请输入有效的11位手机号").optional().or(z.literal("")),
});

onMounted(fetchTenants);

async function fetchTenants() {
  loading.value = true;
  try {
    tenants.value = await tenantService.list();
  } catch {
    ElMessage.error("获取租客列表失败");
  } finally {
    loading.value = false;
  }
}

const filteredTenants = computed(() => {
  if (!uiStore.globalSearchKeyword) return tenants.value;
  const kw = uiStore.globalSearchKeyword.toLowerCase();
  return tenants.value.filter(
    (t) =>
      t.name.toLowerCase().includes(kw) ||
      t.phone.includes(kw)
  );
});

function openCreateDialog() {
  isEditing.value = false;
  editingId.value = 0;
  formData.value = { name: "", phone: "", phone2: "", emergency_contact: "", emergency_phone: "" };
  dialogVisible.value = true;
}

function openEditDialog(row: TenantResponse) {
  isEditing.value = true;
  editingId.value = row.id;
  formData.value = {
    name: row.name,
    phone: row.phone,
    phone2: row.phone2 || "",
    emergency_contact: row.emergency_contact || "",
    emergency_phone: row.emergency_phone || "",
  };
  dialogVisible.value = true;
}

async function handleSubmit() {
  const result = tenantSchema.safeParse(formData.value);
  if (!result.success) {
    ElMessage.error(result.error.issues[0]?.message || "表单验证失败");
    return;
  }
  try {
    if (isEditing.value) {
      const req: UpdateTenantRequest = {
        name: result.data.name,
        phone: result.data.phone,
        phone2: result.data.phone2 || undefined,
        emergency_contact: result.data.emergency_contact || undefined,
        emergency_phone: result.data.emergency_phone || undefined,
      };
      await tenantService.update(editingId.value, req);
      ElMessage.success("更新成功");
    } else {
      await tenantService.create(formData.value);
      ElMessage.success("创建成功");
    }
    dialogVisible.value = false;
    await fetchTenants();
  } catch {
    ElMessage.error(isEditing.value ? "更新失败" : "创建失败");
  }
}

async function handleDelete(row: TenantResponse) {
  try {
    await ElMessageBox.confirm(`确定删除租客 "${row.name}" 吗？有生效合同的租客无法删除。`, "确认删除", {
      type: "warning",
    });
    const deleted = await tenantService.delete(row.id);
    if (deleted) {
      ElMessage.success("删除成功");
      await fetchTenants();
    } else {
      ElMessage.error("删除失败");
    }
  } catch {
    // cancelled or error
  }
}
</script>

<template>
  <div class="tenant-list">
    <el-row :gutter="16" class="toolbar">
      <el-col :span="24" class="toolbar-buttons">
        <el-button type="primary" @click="openCreateDialog">新建租客</el-button>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">租客总数</span>
            <span class="stat-value">{{ tenants.length }}</span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-table :data="filteredTenants" v-loading="loading" stripe>
      <el-table-column prop="name" label="姓名" width="120" />
      <el-table-column prop="phone" label="手机号" width="140" />
      <el-table-column prop="phone2" label="备用手机" width="140">
        <template #default="{ row }">{{ row.phone2 || "-" }}</template>
      </el-table-column>
      <el-table-column prop="emergency_contact" label="紧急联系人" width="120">
        <template #default="{ row }">{{ row.emergency_contact || "-" }}</template>
      </el-table-column>
      <el-table-column prop="emergency_phone" label="紧急联系电话" width="140">
        <template #default="{ row }">{{ row.emergency_phone || "-" }}</template>
      </el-table-column>
      <el-table-column label="操作" fixed="right" width="180">
        <template #default="{ row }">
          <el-button type="primary" size="small" text @click="openEditDialog(row)">编辑</el-button>
          <el-button type="danger" size="small" text @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="dialogVisible" :title="isEditing ? '编辑租客' : '新建租客'" width="500px">
      <el-form :model="formData" label-width="100px">
        <el-form-item label="姓名">
          <el-input v-model="formData.name" placeholder="请输入姓名" />
        </el-form-item>
        <el-form-item label="手机号">
          <el-input v-model="formData.phone" placeholder="请输入手机号" />
        </el-form-item>
        <el-form-item label="备用手机">
          <el-input v-model="formData.phone2" placeholder="请输入备用手机号" />
        </el-form-item>
        <el-form-item label="紧急联系人">
          <el-input v-model="formData.emergency_contact" placeholder="请输入紧急联系人" />
        </el-form-item>
        <el-form-item label="紧急联系电话">
          <el-input v-model="formData.emergency_phone" placeholder="请输入紧急联系电话" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.tenant-list {
  .toolbar {
    margin-top: 16px;
    margin-bottom: 16px;

    .toolbar-buttons {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
    }
  }

  .stats-row {
    margin-bottom: 16px;

    .stat-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px 0;

      .stat-label {
        font-size: 14px;
        color: var(--el-text-color-secondary);
        margin-bottom: 4px;
      }

      .stat-value {
        font-size: 24px;
        font-weight: bold;
        color: var(--el-color-primary);
      }
    }
  }
}
</style>
