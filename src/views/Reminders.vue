<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { reminderService } from "@/services/reminder.service";
import type {
  Reminder,
  CreateReminderRequest,
} from "@/services/reminder.service";

const loading = ref(false);
const reminders = ref<Reminder[]>([]);
const filterRead = ref<boolean | undefined>(undefined);

// 创建对话框
const createDialogVisible = ref(false);
const createForm = ref<CreateReminderRequest>({
  reminder_type: "",
  title: "",
  message: "",
  room_id: undefined,
  lease_id: undefined,
  scheduled_date: "",
});

const reminderTypeOptions = [
  { label: "租金到期提醒", value: "租金到期" },
  { label: "合同到期提醒", value: "合同到期" },
  { label: "账单提醒", value: "账单" },
  { label: "退房提醒", value: "退房" },
  { label: "其他", value: "其他" },
];

const readStatusOptions = [
  { label: "全部", value: undefined },
  { label: "未读", value: false },
  { label: "已读", value: true },
];

onMounted(async () => {
  await fetchReminders();
});

async function fetchReminders() {
  loading.value = true;
  try {
    const resp = await reminderService.list(undefined, filterRead.value);
    if (resp.success) {
      reminders.value = resp.data || [];
    } else {
      ElMessage.error(resp.message || "获取提醒失败");
    }
  } catch (e) {
    ElMessage.error("获取提醒失败");
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function onFilterChange() {
  await fetchReminders();
}

function openCreateDialog() {
  createForm.value = {
    reminder_type: "",
    title: "",
    message: "",
    room_id: undefined,
    lease_id: undefined,
    scheduled_date: "",
  };
  createDialogVisible.value = true;
}

async function submitCreate() {
  if (!createForm.value.reminder_type || !createForm.value.title) {
    ElMessage.warning("请填写必填项");
    return;
  }
  try {
    const resp = await reminderService.create(createForm.value);
    if (resp.success) {
      ElMessage.success("创建成功");
      createDialogVisible.value = false;
      await fetchReminders();
    } else {
      ElMessage.error(resp.message || "创建失败");
    }
  } catch (e) {
    ElMessage.error("创建失败");
    console.error(e);
  }
}

async function markAsRead(id: number) {
  try {
    const resp = await reminderService.markRead(id);
    if (resp.success) {
      await fetchReminders();
    } else {
      ElMessage.error(resp.message || "操作失败");
    }
  } catch (e) {
    ElMessage.error("操作失败");
    console.error(e);
  }
}

async function markAsSent(id: number) {
  try {
    const resp = await reminderService.markSent(id);
    if (resp.success) {
      await fetchReminders();
    } else {
      ElMessage.error(resp.message || "操作失败");
    }
  } catch (e) {
    ElMessage.error("操作失败");
    console.error(e);
  }
}

async function deleteReminder(id: number) {
  try {
    await ElMessageBox.confirm("确定删除此提醒？", "确认", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    const resp = await reminderService.delete(id);
    if (resp.success) {
      ElMessage.success("删除成功");
      await fetchReminders();
    } else {
      ElMessage.error(resp.message || "删除失败");
    }
  } catch (e: any) {
    if (e !== "cancel") {
      ElMessage.error("删除失败");
      console.error(e);
    }
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "-";
  return dateStr.slice(0, 16);
}

function getTypeTagType(type: string): "primary" | "success" | "warning" | "info" | "danger" {
  const map: Record<string, "primary" | "success" | "warning" | "info" | "danger"> = {
    租金到期: "warning",
    合同到期: "primary",
    账单: "success",
    退房: "danger",
    其他: "info",
  };
  return map[type] || "info";
}
</script>

<template>
  <div class="reminders">
    <!-- 操作栏 -->
    <el-card class="tool-card">
      <div class="toolbar">
        <el-radio-group v-model="filterRead" @change="onFilterChange">
          <el-radio-button
            v-for="opt in readStatusOptions"
            :key="String(opt.value)"
            :label="opt.value"
          >
            {{ opt.label }}
          </el-radio-button>
        </el-radio-group>
        <el-button type="primary" @click="openCreateDialog">新建提醒</el-button>
      </div>
    </el-card>

    <!-- 提醒列表 -->
    <el-card class="reminder-card" v-loading="loading">
      <template #header>
        <div class="card-header">
          <span>提醒列表</span>
        </div>
      </template>

      <el-table :data="reminders" stripe>
        <el-table-column prop="reminder_type" label="类型" width="120">
          <template #default="{ row }">
            <el-tag :type="getTypeTagType(row.reminder_type)" size="small">
              {{ row.reminder_type }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="title" label="标题" min-width="150" />
        <el-table-column prop="message" label="内容" min-width="200">
          <template #default="{ row }">
            {{ row.message || "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="room_id" label="房间" width="80">
          <template #default="{ row }">
            {{ row.room_id || "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="scheduled_date" label="计划日期" width="160">
          <template #default="{ row }">
            {{ formatDate(row.scheduled_date) }}
          </template>
        </el-table-column>
        <el-table-column prop="is_sent" label="已发送" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_sent ? 'success' : 'info'" size="small">
              {{ row.is_sent ? "是" : "否" }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="isRead" label="已读" width="80">
          <template #default="{ row }">
            <el-tag :type="row.isRead ? 'success' : 'warning'" size="small">
              {{ row.isRead ? "是" : "否" }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180">
          <template #default="{ row }">
            <el-button
              v-if="!row.isRead"
              type="primary"
              link
              size="small"
              @click="markAsRead(row.id)"
            >
              标记已读
            </el-button>
            <el-button
              v-if="!row.isSent"
              type="success"
              link
              size="small"
              @click="markAsSent(row.id)"
            >
              标记已发送
            </el-button>
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteReminder(row.id)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="reminders.length === 0 && !loading" class="empty-tip">
        <el-empty description="暂无提醒数据" />
      </div>
    </el-card>

    <!-- 创建对话框 -->
    <el-dialog v-model="createDialogVisible" title="新建提醒" width="500px">
      <el-form :model="createForm" label-width="100px">
        <el-form-item label="类型" required>
          <el-select v-model="createForm.reminder_type" placeholder="选择类型">
            <el-option
              v-for="opt in reminderTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="标题" required>
          <el-input v-model="createForm.title" placeholder="提醒标题" />
        </el-form-item>
        <el-form-item label="内容">
          <el-input
            v-model="createForm.message"
            type="textarea"
            :rows="3"
            placeholder="提醒内容"
          />
        </el-form-item>
        <el-form-item label="房间ID">
          <el-input-number
            v-model="createForm.room_id"
            :min="0"
            placeholder="关联房间"
          />
        </el-form-item>
        <el-form-item label="合同ID">
          <el-input-number
            v-model="createForm.lease_id"
            :min="0"
            placeholder="关联合同"
          />
        </el-form-item>
        <el-form-item label="计划日期">
          <el-date-picker
            v-model="createForm.scheduled_date"
            type="datetime"
            placeholder="选择日期时间"
            value-format="YYYY-MM-DD HH:mm:ss"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitCreate">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.reminders {
  .tool-card {
    margin-top: 16px;
  }

  .reminder-card {
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

  .empty-tip {
    margin-top: 20px;
  }
}
</style>
