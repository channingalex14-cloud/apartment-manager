<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { configService } from "@/services/config.service";
import type { SystemConfig } from "@/types/config";

const loading = ref(false);
const configs = ref<SystemConfig[]>([]);

async function fetchConfigs() {
  loading.value = true;
  try {
    configs.value = await configService.getAll();
  } catch {
    ElMessage.error('获取配置失败');
  } finally {
    loading.value = false;
  }
}

onMounted(fetchConfigs);
</script>

<template>
  <el-card style="margin-top: 20px" v-loading="loading">
    <template #header>
      <div class="card-header">
        <span>全部配置项</span>
      </div>
    </template>

    <el-table :data="configs" stripe>
      <el-table-column prop="config_key" label="配置键" width="220" />
      <el-table-column prop="config_value" label="配置值" width="200">
        <template #default="{ row }">
          {{ row.config_value || '-' }}
        </template>
      </el-table-column>
      <el-table-column prop="config_type" label="类型" width="120">
        <template #default="{ row }">
          {{ row.config_type || '-' }}
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述">
        <template #default="{ row }">
          {{ row.description || '-' }}
        </template>
      </el-table-column>
      <el-table-column label="状态" width="80">
        <template #default="{ row }">
          <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
            {{ row.is_active ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="configs.length === 0" class="empty-tip">
      <el-empty description="暂无配置数据" />
    </div>
  </el-card>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.empty-tip {
  margin-top: 20px;
}
</style>
