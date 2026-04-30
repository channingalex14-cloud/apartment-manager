<template>
  <div class="user-management-page">
    <el-card shadow="hover">
      <template #header>
        <div class="card-header">
          <span>用户管理</span>
          <el-button type="primary" size="small" @click="showCreateDialog">
            新增用户
          </el-button>
        </div>
      </template>

      <el-table
        :data="users"
        stripe
        size="small"
        v-loading="loading"
        max-height="600"
      >
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="username" label="用户名" width="140" />
        <el-table-column prop="displayName" label="显示名称" width="140">
          <template #default="{ row }">
            {{ row.displayName || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="role" label="角色" width="120">
          <template #default="{ row }">
            <el-tag :type="roleTagType(row.role)" size="small">
              {{ roleLabel(row.role) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="isActive" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.isActive ? 'success' : 'danger'" size="small">
              {{ row.isActive ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" min-width="240">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="showEditDialog(row)"
            >
              编辑
            </el-button>
            <el-button
              type="warning"
              link
              size="small"
              @click="handleResetPassword(row)"
            >
              重置密码
            </el-button>
            <el-button
              v-if="row.isActive"
              type="info"
              link
              size="small"
              @click="handleToggleActive(row, false)"
            >
              禁用
            </el-button>
            <el-button
              v-else
              type="success"
              link
              size="small"
              @click="handleToggleActive(row, true)"
            >
              启用
            </el-button>
            <el-button
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑用户' : '新增用户'"
      width="480px"
      :close-on-click-modal="false"
    >
      <el-form
        :model="formData"
        label-width="100px"
        :rules="formRules"
        ref="formRef"
      >
        <el-form-item label="用户名" prop="username" v-if="!isEditing">
          <el-input v-model="formData.username" placeholder="请输入用户名" />
        </el-form-item>
        <el-form-item label="用户名" v-else>
          <el-input :model-value="formData.username" disabled />
        </el-form-item>
        <el-form-item label="密码" prop="password" v-if="!isEditing">
          <el-input
            v-model="formData.password"
            type="password"
            show-password
            placeholder="请输入密码"
          />
        </el-form-item>
        <el-form-item label="角色" prop="role">
          <el-select v-model="formData.role" placeholder="请选择角色">
            <el-option label="管理员" value="admin" />
            <el-option label="前台" value="frontdesk" />
            <el-option label="维护" value="maintenance" />
            <el-option label="财务" value="finance" />
          </el-select>
        </el-form-item>
        <el-form-item label="显示名称" prop="displayName">
          <el-input v-model="formData.displayName" placeholder="可选" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitting">
          确定
        </el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="resetPwdVisible"
      title="重置密码"
      width="400px"
      :close-on-click-modal="false"
    >
      <el-form :model="resetPwdData" label-width="80px" :rules="resetPwdRules" ref="resetPwdFormRef">
        <el-form-item label="用户">
          <el-input :model-value="resetPwdTarget" disabled />
        </el-form-item>
        <el-form-item label="新密码" prop="newPassword">
          <el-input
            v-model="resetPwdData.newPassword"
            type="password"
            show-password
            placeholder="请输入新密码"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="resetPwdVisible = false">取消</el-button>
        <el-button type="warning" @click="handleResetPwdSubmit" :loading="submitting">
          确认重置
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import type { FormInstance, FormRules } from "element-plus";
import {
  authService,
  type User,
  type CreateUserRequest,
  type UpdateUserRequest,
} from "@/services/auth.service";

const users = ref<User[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const isEditing = ref(false);
const submitting = ref(false);
const editingId = ref<number>(0);
const formRef = ref<FormInstance>();

const formData = ref<{
  username: string;
  password: string;
  role: string;
  displayName: string;
}>({
  username: "",
  password: "",
  role: "frontdesk",
  displayName: "",
});

const formRules: FormRules = {
  username: [{ required: true, message: "请输入用户名", trigger: "blur" }],
  password: [
    { required: true, message: "请输入密码", trigger: "blur" },
    { min: 6, message: "密码至少6位", trigger: "blur" },
  ],
  role: [{ required: true, message: "请选择角色", trigger: "change" }],
};

const resetPwdVisible = ref(false);
const resetPwdTarget = ref("");
const resetPwdTargetId = ref(0);
const resetPwdFormRef = ref<FormInstance>();
const resetPwdData = ref({ newPassword: "" });

const resetPwdRules: FormRules = {
  newPassword: [
    { required: true, message: "请输入新密码", trigger: "blur" },
    { min: 6, message: "密码至少6位", trigger: "blur" },
  ],
};

function roleLabel(role: string): string {
  const map: Record<string, string> = {
    admin: "管理员",
    frontdesk: "前台",
    maintenance: "维护",
    finance: "财务",
  };
  return map[role] || role;
}

function roleTagType(role: string): "primary" | "success" | "warning" | "danger" | "info" {
  const map: Record<string, "primary" | "success" | "warning" | "danger" | "info"> = {
    admin: "danger",
    frontdesk: "primary",
    maintenance: "warning",
    finance: "success",
  };
  return map[role] || "info";
}

async function fetchUsers() {
  loading.value = true;
  try {
    users.value = await authService.listUsers();
  } catch {
    ElMessage.error("加载用户列表失败");
  } finally {
    loading.value = false;
  }
}

function showCreateDialog() {
  isEditing.value = false;
  editingId.value = 0;
  formData.value = {
    username: "",
    password: "",
    role: "frontdesk",
    displayName: "",
  };
  dialogVisible.value = true;
}

function showEditDialog(user: User) {
  isEditing.value = true;
  editingId.value = user.id;
  formData.value = {
    username: user.username,
    password: "",
    role: user.role,
    displayName: user.displayName || "",
  };
  dialogVisible.value = true;
}

async function handleSubmit() {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    if (isEditing.value) {
      const req: UpdateUserRequest = {
        role: formData.value.role,
        displayName: formData.value.displayName || undefined,
      };
      const res = await authService.updateUser(editingId.value, req);
      if (res.success) {
        ElMessage.success("用户更新成功");
        dialogVisible.value = false;
        await fetchUsers();
      } else {
        ElMessage.error(res.message);
      }
    } else {
      const req: CreateUserRequest = {
        username: formData.value.username,
        password: formData.value.password,
        role: formData.value.role,
        displayName: formData.value.displayName || undefined,
      };
      const res = await authService.createUser(req);
      if (res.success) {
        ElMessage.success("用户创建成功");
        dialogVisible.value = false;
        await fetchUsers();
      } else {
        ElMessage.error(res.message);
      }
    }
  } catch {
    ElMessage.error("操作失败");
  } finally {
    submitting.value = false;
  }
}

async function handleResetPassword(user: User) {
  resetPwdTarget.value = user.username;
  resetPwdTargetId.value = user.id;
  resetPwdData.value = { newPassword: "" };
  resetPwdVisible.value = true;
}

async function handleResetPwdSubmit() {
  if (!resetPwdFormRef.value) return;
  const valid = await resetPwdFormRef.value.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    const res = await authService.resetPassword(
      resetPwdTargetId.value,
      resetPwdData.value.newPassword,
    );
    if (res.success) {
      ElMessage.success("密码重置成功");
      resetPwdVisible.value = false;
    } else {
      ElMessage.error(res.message);
    }
  } catch {
    ElMessage.error("重置密码失败");
  } finally {
    submitting.value = false;
  }
}

async function handleToggleActive(user: User, active: boolean) {
  const action = active ? "启用" : "禁用";
  try {
    await ElMessageBox.confirm(
      `确定要${action}用户 "${user.username}" 吗？`,
      "确认操作",
      { type: "warning" },
    );
    const res = await authService.updateUser(user.id, { isActive: active });
    if (res.success) {
      ElMessage.success(`已${action}用户`);
      await fetchUsers();
    } else {
      ElMessage.error(res.message);
    }
  } catch {
    // cancelled
  }
}

async function handleDelete(user: User) {
  try {
    await ElMessageBox.confirm(
      `确定要删除用户 "${user.username}" 吗？此操作不可恢复。`,
      "确认删除",
      { type: "warning" },
    );
    const res = await authService.deleteUser(user.id);
    if (res.success) {
      ElMessage.success("用户已删除");
      await fetchUsers();
    } else {
      ElMessage.error(res.message);
    }
  } catch {
    // cancelled
  }
}

onMounted(fetchUsers);
</script>

<style scoped lang="scss">
.user-management-page {
  padding: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
