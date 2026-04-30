<script setup lang="ts">
import { ref, onErrorCaptured, type ComponentPublicInstance } from 'vue'

const hasError = ref(false)
const errorMessage = ref('')
const errorStack = ref('')

// 捕获子组件错误
onErrorCaptured((error: Error, instance: ComponentPublicInstance | null, info: string) => {
  hasError.value = true
  errorMessage.value = error.message || '未知错误'
  errorStack.value = error.stack || ''

  // 记录错误日志（后续可接入日志系统）
  console.error('[ErrorBoundary] 捕获到错误:', {
    error,
    component: instance?.$options?.name || '未知组件',
    info,
  })

  // 返回 false 阻止错误继续向上传播
  return false
})

// 重试功能
function retry() {
  hasError.value = false
  errorMessage.value = ''
  errorStack.value = ''
}

// 复制错误信息
function copyError() {
  const text = `错误信息: ${errorMessage.value}\n\n堆栈追踪:\n${errorStack.value}`
  navigator.clipboard.writeText(text)
}
</script>

<template>
  <div v-if="hasError" class="error-boundary">
    <div class="error-content">
      <el-icon :size="64" color="#f56c6c">
        <WarningFilled />
      </el-icon>
      <h2>页面出错了</h2>
      <p class="error-message">{{ errorMessage }}</p>
      <div class="error-actions">
        <el-button type="primary" @click="retry">
          重试
        </el-button>
        <el-button @click="copyError">
          复制错误信息
        </el-button>
      </div>
      <el-collapse v-if="errorStack" class="error-stack">
        <el-collapse-item title="查看详细信息" name="stack">
          <pre>{{ errorStack }}</pre>
        </el-collapse-item>
      </el-collapse>
    </div>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
  padding: 24px;
}

.error-content {
  text-align: center;
  max-width: 600px;
}

.error-content h2 {
  margin: 16px 0 8px;
  font-size: 20px;
  color: #303133;
}

.error-message {
  color: #909399;
  margin-bottom: 24px;
}

.error-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
  margin-bottom: 24px;
}

.error-stack {
  text-align: left;
}

.error-stack pre {
  font-size: 12px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
