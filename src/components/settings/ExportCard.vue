<script setup lang="ts">
import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { exportService, type ExportType } from "@/services/export.service";

const exporting = ref(false);
const exportType = ref<ExportType>("rooms");
const exportYearMonth = ref("");

function getExportTypeLabel(type: ExportType): string {
  const labels: Record<ExportType, string> = {
    rooms: "房间数据",
    tenants: "租客数据",
    bills: "账单数据",
    payments: "缴费记录",
    summary: "年度汇总",
  };
  return labels[type] || type;
}

async function handleExportData() {
  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要导出 "${getExportTypeLabel(exportType.value)}" 数据吗？${exportYearMonth.value ? `\n年月: ${exportYearMonth.value}` : ''}`,
      "确认导出",
      { type: "info" }
    ).catch(() => false);

    if (!confirmed) return;

    exporting.value = true;

    const jsonStr = await exportService.exportData(exportType.value, exportYearMonth.value || undefined);

    const timestamp = new Date().toISOString().slice(0, 10).replace(/-/g, "");
    const defaultName = `${exportType.value}_${exportYearMonth.value || timestamp}.json`;

    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "JSON 文件", extensions: ["json"] }],
    });

    if (!filePath) {
      ElMessage.warning("已取消导出");
      return;
    }

    await writeTextFile(filePath, jsonStr);
    ElMessage.success(`导出成功: ${filePath}`);
  } catch (e: any) {
    ElMessage.error("导出失败: " + (e?.message || e));
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <el-card style="margin-top: 20px">
    <template #header>
      <div class="card-header">
        <span>数据导出</span>
        <el-tag size="small" type="info">JSON 格式</el-tag>
      </div>
    </template>
    <div class="export-form">
      <el-alert type="info" :closable="false" style="margin-bottom: 16px">
        <template #title>
          导出数据为 JSON 格式文件，可用于数据备份或迁移到其他系统。<br />
          导出的文件为 UTF-8 编码，可直接用 Excel 或文本编辑器打开。
        </template>
      </el-alert>
      <el-form label-width="100px">
        <el-form-item label="导出类型">
          <el-select v-model="exportType" style="width: 200px">
            <el-option label="房间数据" value="rooms" />
            <el-option label="租客数据" value="tenants" />
            <el-option label="账单数据" value="bills" />
            <el-option label="缴费记录" value="payments" />
            <el-option label="年度汇总" value="summary" />
          </el-select>
        </el-form-item>
        <el-form-item label="年月(可选)">
          <el-date-picker v-model="exportYearMonth" type="month" placeholder="筛选年月（可选）" value-format="YYYY-MM" style="width: 200px" clearable />
        </el-form-item>
        <el-form-item>
          <el-button type="success" :loading="exporting" @click="handleExportData">
            导出 JSON
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
