<template>
  <div class="css-diagnostic">
    <h3>CSS 变量诊断工具</h3>

    <div class="diagnostic-section">
      <h4>Element Plus 主色变量</h4>
      <div class="var-grid">
        <div class="var-item" v-for="(value, name) in epColors" :key="name">
          <span class="var-name">{{ name }}</span>
          <span class="var-value" :style="{ color: value, background: getBgColor(value) }">
            {{ value }}
          </span>
          <span class="color-swatch" :style="{ background: value }"></span>
        </div>
      </div>
    </div>

    <div class="diagnostic-section">
      <h4>自定义房态颜色变量</h4>
      <div class="var-grid">
        <div class="var-item" v-for="(value, name) in roomColors" :key="name">
          <span class="var-name">{{ name }}</span>
          <span class="var-value" :style="{ color: value, background: getBgColor(value) }">
            {{ value }}
          </span>
          <span class="color-swatch" :style="{ background: value }"></span>
        </div>
      </div>
    </div>

    <div class="diagnostic-section">
      <h4>布局专用色变量</h4>
      <div class="var-grid">
        <div class="var-item" v-for="(value, name) in layoutColors" :key="name">
          <span class="var-name">{{ name }}</span>
          <span class="var-value">{{ value }}</span>
          <span class="color-swatch" :style="{ background: value }"></span>
        </div>
      </div>
    </div>

    <div class="actions">
      <el-button type="primary" @click="refresh">刷新检测</el-button>
      <el-button @click="copyToClipboard">复制结果</el-button>
      <el-button type="danger" @click="$emit('close')">关闭</el-button>
    </div>

    <div class="status-message" :class="{ success: isCorrect, error: !isCorrect }">
      {{ isCorrect ? '所有CSS变量已正确加载！' : '检测到部分变量未生效' }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const emit = defineEmits(['close'])

const epColors = ref<Record<string, string>>({})
const roomColors = ref<Record<string, string>>({})
const layoutColors = ref<Record<string, string>>({})
const isCorrect = ref(false)

/** 读取所有CSS变量的实际值 */
function readAllVariables() {
  const style = getComputedStyle(document.documentElement)

  // EP主色
  epColors.value = {
    '--el-color-primary': style.getPropertyValue('--el-color-primary').trim(),
    '--el-color-success': style.getPropertyValue('--el-color-success').trim(),
    '--el-color-warning': style.getPropertyValue('--el-color-warning').trim(),
    '--el-color-danger': style.getPropertyValue('--el-color-danger').trim(),
    '--el-color-info': style.getPropertyValue('--el-color-info').trim(),
  }

  // 房态颜色
  roomColors.value = {
    '--room-rented': style.getPropertyValue('--room-rented').trim(),
    '--room-new': style.getPropertyValue('--room-new').trim(),
    '--room-vacant': style.getPropertyValue('--room-vacant').trim(),
    '--room-manage': style.getPropertyValue('--room-manage').trim(),
  }

  // 布局色
  layoutColors.value = {
    '--sidebar-bg': style.getPropertyValue('--sidebar-bg').trim(),
    '--header-bg': style.getPropertyValue('--header-bg').trim(),
    '--main-bg': style.getPropertyValue('--main-bg').trim(),
    '--bg-card': style.getPropertyValue('--bg-card').trim(),
  }

  // 验证是否正确（兼容浅色和深色模式）
  const primaryVal = epColors.value['--el-color-primary']
  isCorrect.value = primaryVal === '#2C73D2' || primaryVal === '#5A9BE6'
}

function refresh() {
  readAllVariables()
}

function getBgColor(color: string): string {
  if (!color || color === 'transparent') return '#fff'
  return `${color}20`
}

async function copyToClipboard() {
  const text = `
=== EP主色 ===
${JSON.stringify(epColors.value, null, 2)}

=== 房态色 ===
${JSON.stringify(roomColors.value, null, 2)}

=== 布局色 ===
${JSON.stringify(layoutColors.value, null, 2)}
  `.trim()

  try {
    await navigator.clipboard.writeText(text)
    alert('已复制到剪贴板！')
  } catch (err) {
    console.error('复制失败:', err)
  }
}

onMounted(() => {
  readAllVariables()
})
</script>

<style scoped lang="scss">
.css-diagnostic {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: var(--el-bg-color);
  padding: 24px;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  z-index: 9999;
  max-width: 800px;
  max-height: 80vh;
  overflow-y: auto;

  h3 {
    margin: 0 0 20px;
    text-align: center;
    color: var(--el-text-color-primary);
  }
}

.diagnostic-section {
  margin-bottom: 20px;

  h4 {
    margin: 0 0 12px;
    color: var(--el-text-color-regular);
    font-size: 14px;
  }
}

.var-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 8px;
}

.var-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  font-size: 13px;

  .var-name {
    flex: 0 0 140px;
    font-family: monospace;
    color: var(--el-text-color-secondary);
  }

  .var-value {
    flex: 1;
    font-family: monospace;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .color-swatch {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    border: 1px solid var(--el-border-color);
    flex-shrink: 0;
  }
}

.actions {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--el-border-color-lighter);
}

.status-message {
  margin-top: 16px;
  padding: 12px;
  border-radius: 6px;
  text-align: center;
  font-weight: 600;

  &.success {
    background: var(--el-color-success-light-9);
    color: var(--el-color-success);
    border: 1px solid var(--el-color-success-light-8);
  }

  &.error {
    background: var(--el-color-danger-light-9);
    color: var(--el-color-danger);
    border: 1px solid var(--el-color-danger-light-8);
  }
}
</style>
