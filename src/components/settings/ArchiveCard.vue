<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { billService } from "@/services/bill.service";

const archiving = ref(false);
const archivingYearMonth = ref("");
const archivedMonths = ref<string[]>([]);

async function fetchArchivedMonths() {
  try {
    archivedMonths.value = await billService.listArchivedMonths();
  } catch {
    console.error("获取已归档年月失败");
  }
}

async function handleArchive() {
  if (!archivingYearMonth.value) {
    ElMessage.warning("请选择要归档的年月");
    return;
  }

  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要归档 ${archivingYearMonth.value} 的所有账单吗？\n\n归档后这些账单将默认不显示，但数据不会丢失，可以随时恢复。`,
      "确认归档",
      { type: "warning" }
    ).catch(() => false);

    if (!confirmed) return;

    archiving.value = true;
    const result = await billService.archiveBills(archivingYearMonth.value);

    if (result.success) {
      ElMessage.success(`归档成功，共归档 ${result.archived_count} 条账单`);
      archivingYearMonth.value = "";
      await fetchArchivedMonths();
    } else {
      ElMessage.error(result.message || "归档失败");
    }
  } catch (e: any) {
    ElMessage.error("归档失败: " + (e?.message || e));
  } finally {
    archiving.value = false;
  }
}

async function handleRestore(yearMonth: string) {
  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要恢复 ${yearMonth} 的归档账单吗？\n\n恢复后这些账单将重新显示在账单列表中。`,
      "确认恢复",
      { type: "info" }
    ).catch(() => false);

    if (!confirmed) return;

    const result = await billService.restoreBills(yearMonth);

    if (result.success) {
      ElMessage.success(`恢复成功，共恢复 ${result.archived_count} 条账单`);
      await fetchArchivedMonths();
    } else {
      ElMessage.error(result.message || "恢复失败");
    }
  } catch (e: any) {
    ElMessage.error("恢复失败: " + (e?.message || e));
  }
}

onMounted(fetchArchivedMonths);
</script>

<template>
  <el-card style="margin-top: 20px">
    <template #header>
      <div class="card-header">
        <span>归档历史账单</span>
        <el-tag size="small" type="info">数据安全</el-tag>
      </div>
    </template>
    <div class="archive-form">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        <template #title>
          归档可以将历史月份的账单标记为"已归档"状态，默认不再显示但数据不会丢失。<br />
          建议将超过 12 个月的历史账单进行归档，以提升系统性能。
        </template>
      </el-alert>
      <el-form label-width="100px">
        <el-form-item label="选择年月">
          <el-date-picker
            v-model="archivingYearMonth"
            type="month"
            placeholder="选择要归档的年月"
            value-format="YYYY-MM"
            :disabled-date="(date: Date) => {
              const now = new Date();
              return date.getTime() > new Date(now.getFullYear(), now.getMonth(), 1).getTime();
            }"
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="warning" :loading="archiving" :disabled="!archivingYearMonth" @click="handleArchive">
            归档选中月份
          </el-button>
        </el-form-item>
      </el-form>

      <div v-if="archivedMonths.length > 0" class="archived-list">
        <el-divider content-position="left">已归档月份</el-divider>
        <el-table :data="archivedMonths" stripe size="small" max-height="200">
          <el-table-column prop="" label="年月" width="120">
            <template #default="{ row }">
              {{ row }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="120">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click="handleRestore(row)">
                恢复
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
      <el-empty v-else description="暂无已归档的账单" :image-size="60" />
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.archived-list {
  margin-top: 16px;
}
</style>
