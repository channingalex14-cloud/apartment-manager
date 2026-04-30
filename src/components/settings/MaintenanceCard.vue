<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";
import { maintenanceService } from "@/services/maintenance.service";

const vacuuming = ref(false);

async function doVacuum() {
  vacuuming.value = true;
  try {
    const result = await maintenanceService.vacuumDatabase();
    ElMessage.success(result || '数据库压缩完成');
  } catch {
    ElMessage.error('压缩失败');
  } finally {
    vacuuming.value = false;
  }
}
</script>

<template>
  <el-card style="margin-top: 20px">
    <template #header>
      <div class="card-header">
        <span>数据维护</span>
        <el-tag size="small" type="danger">谨慎操作</el-tag>
      </div>
    </template>
    <div class="maintenance-tip">
      <el-alert type="info" :closable="false">
        <template #title>
          提示：数据库压缩（VACUUM）可以回收删除记录后未释放的空间，建议每月执行一次。操作可能需要几秒钟。
        </template>
      </el-alert>
      <el-button type="warning" style="margin-top: 16px" :loading="vacuuming" @click="doVacuum">
        执行数据库压缩
      </el-button>
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
