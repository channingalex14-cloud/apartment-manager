<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { configService } from "@/services/config.service";
import type { SystemConfig } from "@/types/config";
import { toCent, toYuanNumber } from "@/utils/money";

const loading = ref(false);
const saving = ref(false);

const form = ref({
  水费单价: undefined as number | undefined,
  电费单价: undefined as number | undefined,
  管理费单价: undefined as number | undefined,
  默认押金: undefined as number | undefined,
  账单到期日: undefined as number | undefined,
  滞纳金比例: undefined as number | undefined,
});

const configItems = [
  { key: '水费单价', label: '水费单价(元/吨)', toDb: (v: number) => String(toCent(v)), fromDb: (v: string) => toYuanNumber(Number(v)) },
  { key: '电费单价', label: '电费单价(元/度)', toDb: (v: number) => String(toCent(v)), fromDb: (v: string) => toYuanNumber(Number(v)) },
  { key: '管理费单价', label: '物业费单价(元/月)', toDb: (v: number) => String(toCent(v)), fromDb: (v: string) => toYuanNumber(Number(v)) },
  { key: '默认押金', label: '默认押金(元)', toDb: (v: number) => String(toCent(v)), fromDb: (v: string) => toYuanNumber(Number(v)) },
  { key: '账单到期日', label: '账单到期日', toDb: (v: number) => String(v), fromDb: (v: string) => Number(v) },
  { key: '滞纳金比例', label: '滞纳金比例(%)', toDb: (v: number) => String(v), fromDb: (v: string) => Number(v) },
];

async function fetchConfigs() {
  loading.value = true;
  try {
    const configs = await configService.getAll();
    configItems.forEach(item => {
      const config = configs.find(c => c.config_key === item.key);
      const val = config?.config_value || '';
      (form.value as any)[item.key] = val === '' ? undefined : item.fromDb(val);
    });
  } catch {
    ElMessage.error('获取配置失败');
  } finally {
    loading.value = false;
  }
}

async function saveAllPrices() {
  saving.value = true;
  try {
    for (const item of configItems) {
      const val = (form.value as any)[item.key];
      if (val !== undefined && val !== null) {
        await configService.set(item.key, item.toDb(val));
      }
    }
    ElMessage.success('保存成功');
    await fetchConfigs();
  } catch {
    ElMessage.error('保存失败');
  } finally {
    saving.value = false;
  }
}

onMounted(fetchConfigs);
</script>

<template>
  <el-row :gutter="20">
    <el-col :span="12">
      <el-card shadow="hover" v-loading="loading">
        <template #header>
          <div class="card-header">
            <span>费用单价</span>
            <el-tag size="small" type="info">修改后实时生效</el-tag>
          </div>
        </template>
        <el-form label-width="120px" class="price-form">
          <el-form-item label="水费单价">
            <el-input-number v-model="form.水费单价" :min="0" :precision="2" :step="0.5" />
            <span class="unit">元/吨</span>
          </el-form-item>
          <el-form-item label="电费单价">
            <el-input-number v-model="form.电费单价" :min="0" :precision="4" :step="0.01" />
            <span class="unit">元/度</span>
          </el-form-item>
          <el-form-item label="物业费单价">
            <el-input-number v-model="form.管理费单价" :min="0" :precision="2" :step="0.01" />
            <span class="unit">元/月</span>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveAllPrices" :loading="saving">保存单价</el-button>
            <el-button @click="fetchConfigs">重置</el-button>
          </el-form-item>
        </el-form>
      </el-card>
    </el-col>

    <el-col :span="12">
      <el-card shadow="hover" v-loading="loading">
        <template #header>
          <div class="card-header">
            <span>账单参数</span>
          </div>
        </template>
        <el-form label-width="120px" class="price-form">
          <el-form-item label="默认押金">
            <el-input-number v-model="form.默认押金" :min="0" :precision="0" :step="100" />
            <span class="unit">元</span>
          </el-form-item>
          <el-form-item label="账单到期日">
            <el-input-number v-model="form.账单到期日" :min="1" :max="31" :precision="0" />
            <span class="unit">日</span>
          </el-form-item>
          <el-form-item label="滞纳金比例">
            <el-input-number v-model="form.滞纳金比例" :min="0" :precision="2" :step="0.1" />
            <span class="unit">% / 天</span>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveAllPrices" :loading="saving">保存参数</el-button>
          </el-form-item>
        </el-form>
      </el-card>
    </el-col>
  </el-row>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.price-form {
  .unit {
    margin-left: 8px;
    font-size: 13px;
    color: var(--el-text-color-secondary);
  }
}
</style>
