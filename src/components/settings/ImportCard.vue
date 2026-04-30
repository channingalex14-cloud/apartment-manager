<script setup lang="ts">
import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { open } from "@tauri-apps/plugin-dialog";
import { importService } from "@/services/import.service";
import { diagnosticService } from "@/services/diagnostic.service";

const importing = ref(false);
const importFilePath = ref("");
const importYearMonth = ref("2026-03");
const importResult = ref<{
  success: boolean;
  imported_count: number;
  skipped_count: number;
  errors: string[];
  message: string | null;
} | null>(null);

async function selectFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Excel 文件", extensions: ["xlsx", "xls"] }],
    });
    if (selected) {
      importFilePath.value = selected as string;
    }
  } catch {
    ElMessage.error("选择文件失败");
  }
}

async function diagnoseExcel() {
  if (!importFilePath.value) {
    ElMessage.warning("请先选择要诊断的 Excel 文件");
    return;
  }
  try {
    const result = await diagnosticService.diagnoseExcel(importFilePath.value);
    let msg = `工作表: ${result.sheet_names.join(", ")}\n`;
    msg += `总行数: ${result.total_rows}\n`;
    msg += `表头列数: ${result.header_row.length}\n`;
    msg += `房号总数: ${result.all_room_numbers.length}\n`;
    msg += `房号样本: ${result.room_numbers_sample.join(", ")}`;
    ElMessageBox.alert(msg, "Excel 诊断结果");
  } catch (e: any) {
    ElMessage.error("诊断失败: " + (e?.message || e));
  }
}

async function handleImport() {
  if (!importFilePath.value) {
    ElMessage.warning('请选择要导入的 Excel 文件');
    return;
  }
  if (!importYearMonth.value) {
    ElMessage.warning('请输入要导入的年月');
    return;
  }

  try {
    const confirmed = await ElMessageBox.confirm(
      `确定要从 "${importFilePath.value}" 导入 ${importYearMonth.value} 的账单数据吗？\n\n注意：已存在的账单会被跳过不会重复创建。`,
      '确认导入',
      { type: 'warning' }
    ).catch(() => false);

    if (!confirmed) return;

    importing.value = true;
    importResult.value = null;

    const result = await importService.importMonthlyBills(importFilePath.value, importYearMonth.value);
    importResult.value = result;

    if (result.success) {
      ElMessage.success(result.message || '导入完成');
    } else {
      ElMessage.error(result.message || '导入部分完成，请检查错误');
    }
  } catch (e: any) {
    ElMessage.error('导入失败: ' + (e?.message || e));
  } finally {
    importing.value = false;
  }
}

function clearImportResult() {
  importResult.value = null;
  importFilePath.value = "";
}
</script>

<template>
  <el-card style="margin-top: 20px">
    <template #header>
      <div class="card-header">
        <span>导入账单</span>
        <el-tag size="small" type="warning">数据导入</el-tag>
      </div>
    </template>
    <div class="import-form">
      <el-form label-width="100px">
        <el-form-item label="Excel文件">
          <el-input v-model="importFilePath" placeholder="点击下方按钮选择 Excel 文件" readonly style="width: 400px" />
          <el-button type="primary" style="margin-left: 12px" @click="selectFile">选择文件</el-button>
          <el-button type="info" style="margin-left: 8px" @click="diagnoseExcel">诊断</el-button>
        </el-form-item>
        <el-form-item label="导入年月">
          <el-date-picker v-model="importYearMonth" type="month" placeholder="选择年月" value-format="YYYY-MM" style="width: 200px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="importing" @click="handleImport">开始导入</el-button>
          <el-button @click="clearImportResult">重置</el-button>
        </el-form-item>
      </el-form>

      <el-alert v-if="importResult" :type="importResult.success ? 'success' : 'warning'" :closable="false" style="margin-top: 16px">
        <template #title>
          <div>{{ importResult.message }}</div>
          <div v-if="importResult.errors.length > 0" style="margin-top: 8px; font-size: 12px;">
            <div v-for="(err, idx) in importResult.errors.slice(0, 10)" :key="idx">
              {{ idx + 1 }}. {{ err }}
            </div>
            <div v-if="importResult.errors.length > 10">
              ... 还有 {{ importResult.errors.length - 10 }} 条错误
            </div>
          </div>
        </template>
      </el-alert>
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
