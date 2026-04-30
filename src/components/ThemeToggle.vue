<template>
  <button
    class="theme-toggle"
    @click="toggleTheme"
    :title="isDark ? '切换到浅色模式' : '切换到深色模式'"
    :aria-label="isDark ? '切换到浅色模式' : '切换到深色模式'"
    role="switch"
    :aria-checked="isDark"
  >
    <!-- 太阳图标（浅色模式） -->
    <svg
      v-if="!isDark"
      class="icon sun-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="12" cy="12" r="5" />
      <line x1="12" y1="1" x2="12" y2="3" />
      <line x1="12" y1="21" x2="12" y2="23" />
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
      <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
      <line x1="1" y1="12" x2="3" y2="12" />
      <line x1="21" y1="12" x2="23" y2="12" />
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
      <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
    </svg>

    <!-- 月亮图标（深色模式） -->
    <svg
      v-else
      class="icon moon-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  </button>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const isDark = ref(false)

onMounted(() => {
  // 初始化时读取当前主题状态
  isDark.value = document.documentElement.getAttribute('data-theme') === 'dark'
})

/** 切换深色/浅色主题 */
function toggleTheme() {
  const nextTheme = isDark.value ? 'light' : 'dark'

  // 使用 View Transitions API 实现平滑过渡（如果支持）
  if (document.startViewTransition) {
    document.startViewTransition(() => {
      applyTheme(nextTheme)
    })
  } else {
    // 降级为普通CSS过渡
    applyTheme(nextTheme)
  }
}

/** 应用主题到DOM */
function applyTheme(theme: string) {
  // 设置 data-theme 属性
  document.documentElement.setAttribute('data-theme', theme)

  // 同步 Element Plus 的 dark 类（EP深色模式需要）
  if (theme === 'dark') {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }

  // 持久化到 localStorage
  localStorage.setItem('apartment-theme', theme)

  // 更新响应式状态
  isDark.value = theme === 'dark'
}
</script>

<style scoped>
.theme-toggle {
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 50%;
  background: var(--window-btn-bg);
  color: var(--window-btn-color);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.25s cubic-bezier(0.645, 0.045, 0.355, 1);
  outline: none;

  &:hover {
    background: var(--window-btn-hover-bg);
    color: var(--window-btn-hover-color);
    transform: scale(1.05);
  }

  &:focus-visible {
    box-shadow: 0 0 0 2px var(--color-primary-light-3);
  }

  .icon {
    width: 18px;
    height: 18px;
    transition: transform 0.3s ease;
  }

  /* 太阳图标旋转动画 */
  .sun-icon {
    transform: rotate(0deg);
  }

  &:hover .sun-icon {
    transform: rotate(45deg);
  }

  /* 月亮图标缩放动画 */
  .moon-icon {
    transform: scale(1);
  }

  &:hover .moon-icon {
    transform: scale(0.9);
  }
}
</style>
