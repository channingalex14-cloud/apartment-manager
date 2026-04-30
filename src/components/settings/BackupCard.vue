<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { backupService } from "@/services/backup.service";

const backingUp = ref(false);
const backups = ref<any[]>([]);
const backupSettings = ref({
  auto_backup_enabled: false,
  retention_count: 7,
  backup_dir: "",
});
const savingBackupSettings = ref(false);

async function fetchBackups() {
  try {
    backups.value = await backupService.listBackups();
  } catch {
    console.error("获取备份列表失败");
  }
}

async function fetchBackupSettings() {
  try {
    backupSettings.value = await backupService.getSettings();
  } catch {
    console.error("获取备份设置失败");
  }
}

async function handleBackupNow() {
  try {
    const confirmed = await ElMessageBox.confirm(
      "确定要立即备份数据库吗？\n\n备份前会先压缩数据库以减小文件体积。",
      "确认备份",
      { type: "warning" }
    ).catch(() => false);

    if (!confirmed) return;

    backingUp.value = true;
    const result = await backupService.backup();

    if (result.success) {
      ElMessage.success(result.message || "备份成功");
      await fetchBackups();
    } else {
      ElMessage.error(result.message || "备份失败");
    }
  } catch (e: any) {
    ElMessage.error("备份失败: " + (e?.message || e));
  } finally {
    backingUp.value = false;
  }
}

async function handleRestoreBackup(backup: any) {
  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要从以下备份恢复数据库吗？\n\n文件名: ${backup.filename}\n创建时间: ${backup.created_at}\n\n警告：恢复操作将覆盖当前数据库，现有数据将丢失！`,
      { title: "确认恢复", type: "warning" }
    ).catch(() => false);

    if (!confirmed) return;

    const result = await backupService.restoreBackup(backup.path);
    ElMessageBox.alert(result + "\n\n请重启应用使恢复生效。", "恢复完成");
  } catch (e: any) {
    ElMessage.error("恢复失败: " + (e?.message || e));
  }
}

async function handleDeleteBackup(backup: any) {
  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要删除以下备份吗？\n\n文件名: ${backup.filename}\n\n此操作不可恢复！`,
      { title: "确认删除", type: "warning" }
    ).catch(() => false);

    if (!confirmed) return;

    await backupService.deleteBackup(backup.path);
    ElMessage.success("备份已删除");
    await fetchBackups();
  } catch (e: any) {
    ElMessage.error("删除失败: " + (e?.message || e));
  }
}

async function saveBackupSettingsHandler() {
  savingBackupSettings.value = true;
  try {
    await backupService.saveSettings(backupSettings.value);
    ElMessage.success("备份设置已保存");
  } catch (e: any) {
    ElMessage.error("保存失败: " + (e?.message || e));
  } finally {
    savingBackupSettings.value = false;
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

onMounted(async () => {
  await fetchBackups();
  await fetchBackupSettings();
});
</script>

<template>
  <el-card style="margin-top: 20px">
    <template #header>
      <div class="card-header">
        <span>数据备份</span>
        <el-tag size="small" type="info">数据安全</el-tag>
      </div>
    </template>
    <div class="backup-form">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        <template #title>
          备份数据库文件到本地目录，支持自动清理旧备份。可在设置中启用自动每日备份。
        </template>
      </el-alert>

      <el-form label-width="140px" inline>
        <el-form-item label="启用自动备份">
          <el-switch v-model="backupSettings.auto_backup_enabled" />
        </el-form-item>
        <el-form-item label="保留份数">
          <el-input-number v-model="backupSettings.retention_count" :min="1" :max="30" :precision="0" style="width: 100px" />
          <span class="unit">份</span>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" size="small" :loading="savingBackupSettings" @click="saveBackupSettingsHandler">
            保存设置
          </el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="backup-actions">
        <el-button type="warning" :loading="backingUp" @click="handleBackupNow">
          立即备份
        </el-button>
        <span class="backup-dir" v-if="backupSettings.backup_dir">
          备份目录: {{ backupSettings.backup_dir }}
        </span>
      </div>

      <div class="backup-list" v-if="backups.length > 0">
        <el-divider content-position="left">备份记录</el-divider>
        <el-table :data="backups" stripe size="small" max-height="200">
          <el-table-column prop="filename" label="文件名" min-width="200" />
          <el-table-column prop="size_bytes" label="大小" width="100">
            <template #default="{ row }">
              {{ formatBytes(row.size_bytes) }}
            </template>
          </el-table-column>
          <el-table-column prop="created_at" label="创建时间" width="160" />
          <el-table-column label="操作" width="160">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click="handleRestoreBackup(row)">恢复</el-button>
              <el-button type="danger" link size="small" @click="handleDeleteBackup(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
      <el-empty v-else description="暂无备份记录" :image-size="60" />
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.unit {
  margin-left: 8px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.backup-form {
  .backup-actions {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
  }

  .backup-dir {
    font-size: 12px;
    color: var(--el-text-color-secondary);
  }

  .backup-list {
    margin-top: 16px;
  }
}
</style>
