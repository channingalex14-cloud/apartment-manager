<script setup lang="ts">
import { onMounted } from "vue";
import { ElConfigProvider } from "element-plus";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import ErrorBoundary from "@/components/ErrorBoundary.vue";
import { logger } from "@/utils/logger";

onMounted(async () => {
  await logger.init();
  logger.info("新逸公寓管理系统 V2.0.5 启动完成");
});
</script>

<template>
  <ErrorBoundary>
    <el-config-provider :locale="zhCn">
      <router-view />
    </el-config-provider>
  </ErrorBoundary>
</template>

<style scoped></style>
