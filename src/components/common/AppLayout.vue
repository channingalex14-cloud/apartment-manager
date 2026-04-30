<template>
  <el-container class="layout-container">
    <!-- 侧边栏 -->
    <el-aside :width="isCollapsed ? '64px' : '210px'" class="sidebar">
      <div class="logo" data-tauri-drag-region @dblclick="toggleMaximize">
        <el-icon class="logo-icon"><OfficeBuilding /></el-icon>
        <span v-if="!isCollapsed">{{ systemStore.aptName }}</span>
      </div>
      <el-menu
        :default-active="currentRoute"
        :collapse="isCollapsed"
        :router="true"
        :background-color="'var(--sidebar-bg)'"
        :text-color="'var(--sidebar-text)'"
        :active-text-color="'var(--sidebar-text-active)'"
      >
        <el-menu-item index="/dashboard">
          <el-icon><Odometer /></el-icon>
          <span>首页概览</span>
        </el-menu-item>
        <el-menu-item index="/rooms">
          <el-icon><House /></el-icon>
          <span>房态管理</span>
        </el-menu-item>
        <el-menu-item index="/tenants">
          <el-icon><User /></el-icon>
          <span>租客管理</span>
        </el-menu-item>
        <el-menu-item index="/bills">
          <el-icon><Tickets /></el-icon>
          <span>账单管理</span>
        </el-menu-item>
        <el-menu-item index="/payments">
          <el-icon><Money /></el-icon>
          <span>缴费管理</span>
        </el-menu-item>
        <el-menu-item v-if="isAdmin" index="/deposits">
          <el-icon><Wallet /></el-icon>
          <span>押金台账</span>
        </el-menu-item>
        <el-menu-item index="/reports">
          <el-icon><DataAnalysis /></el-icon>
          <span>月度报表</span>
        </el-menu-item>
        <el-menu-item index="/documents">
          <el-icon><Folder /></el-icon>
          <span>文档管理</span>
        </el-menu-item>
        <el-menu-item index="/reminders">
          <el-icon><Bell /></el-icon>
          <span>提醒管理</span>
        </el-menu-item>
        <el-menu-item v-if="isAdmin" index="/users">
          <el-icon><UserFilled /></el-icon>
          <span>用户管理</span>
        </el-menu-item>
        <el-menu-item v-if="isAdmin" index="/settings">
          <el-icon><Setting /></el-icon>
          <span>系统设置</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <!-- 顶栏（集成窗口控制按钮和主题切换） -->
      <el-header class="header" height="56px" data-tauri-drag-region>
        <div class="header-left">
          <el-icon class="collapse-btn" @click="isCollapsed = !isCollapsed">
            <Fold v-if="!isCollapsed" />
            <Expand v-else />
          </el-icon>
          <el-breadcrumb separator="/">
            <el-breadcrumb-item>{{ currentMeta?.title || '首页' }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <!-- 全局搜索框（房号/姓名） -->
          <el-input
            :model-value="globalSearch"
            placeholder="搜索房号/姓名"
            style="width: 160px"
            clearable
            @update:model-value="onGlobalSearch"
          />
          <!-- 全局刷新按钮 -->
          <el-button
            @click="onGlobalRefresh"
            title="刷新全部数据"
            style="width: 44px; padding: 0; min-width: 44px;"
          >
            <span style="display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px;">
              <el-icon class="refresh-icon" :class="{ 'is-loading': uiStore.appLoading }">
                <Refresh />
              </el-icon>
            </span>
          </el-button>
          <!-- 全局月份选择器 -->
          <el-date-picker
            v-model="selectedMonth"
            type="month"
            placeholder="选择月份"
            format="YYYY[年]MM[月]"
            value-format="YYYY-MM"
            :clearable="false"
            class="month-picker"
            @change="onMonthChange"
          />
          <!-- 主题切换按钮 -->
          <ThemeToggle />
          <!-- 用户信息和登出 -->
          <div class="user-info">
            <span class="username">{{ currentUsername }}</span>
            <el-button text @click="handleLogout" title="退出登录">
              <el-icon><SwitchButton /></el-icon>
            </el-button>
          </div>
          <!-- 窗口控制按钮 -->
          <div class="window-controls">
            <button class="window-btn minimize" @click="handleMinimizeClick" title="最小化">
              <svg width="12" height="12" viewBox="0 0 12 12">
                <rect x="1" y="5.5" width="10" height="1" fill="currentColor"/>
              </svg>
            </button>
            <button class="window-btn maximize" @click="handleMaximizeClick" title="最大化">
              <svg v-if="!isMaximized" width="12" height="12" viewBox="0 0 12 12">
                <rect x="1" y="1" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1"/>
              </svg>
              <svg v-else width="12" height="12" viewBox="0 0 12 12">
                <rect x="3" y="0" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/>
                <rect x="0" y="3" width="9" height="9" fill="#fff" stroke="currentColor" stroke-width="1"/>
              </svg>
            </button>
            <button class="window-btn close" @click="handleCloseClick" title="关闭">
              <svg width="12" height="12" viewBox="0 0 12 12">
                <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.2"/>
                <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.2"/>
              </svg>
            </button>
          </div>
        </div>
      </el-header>

      <!-- 主内容区 -->
      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>

  <!-- CSS变量诊断工具（按 Ctrl+Shift+D 打开） -->
  <CssDiagnostic v-if="showDiagnostic" @close="showDiagnostic = false" />
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSystemStore, useUIStore, useRoomStore, useBillStore, useTenantStore } from '@/stores'
import { paymentService } from '@/services/payment.service'
import { authService } from '@/services/auth.service'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ThemeToggle from '@/components/ThemeToggle.vue'
import CssDiagnostic from '@/components/CssDiagnostic.vue'
import {
  OfficeBuilding,
  Odometer,
  House,
  Tickets,
  Money,
  Setting,
  Fold,
  Expand,
  Refresh,
  User,
  Bell,
  SwitchButton,
} from '@element-plus/icons-vue'

const route = useRoute()
const router = useRouter()
const systemStore = useSystemStore()
const uiStore = useUIStore()
const roomStore = useRoomStore()
const billStore = useBillStore()
const tenantStore = useTenantStore()

const appWindow = getCurrentWindow()
const isMaximized = ref(false)
const showDiagnostic = ref(false)

async function checkMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized()
  } catch {
    // 非 Tauri 环境（如 Web 开发服务器）忽略
  }
}

function handleMinimizeClick() {
  minimizeWindow()
}

function handleMaximizeClick() {
  toggleMaximize()
}

function handleCloseClick() {
  closeWindow()
}

async function minimizeWindow() {
  try {
    await appWindow.minimize()
  } catch (err) {
    console.error('[WindowControl] 最小化失败:', err)
  }
}

async function toggleMaximize() {
  try {
    await appWindow.toggleMaximize()
    isMaximized.value = await appWindow.isMaximized()
  } catch (err) {
    console.error('[WindowControl] 最大化/还原失败:', err)
  }
}

async function closeWindow() {
  try {
    await appWindow.close()
  } catch (err) {
    console.error('[WindowControl] 关闭窗口失败:', err)
  }
}

async function handleLogout() {
  await authService.logout()
  router.push('/login')
}

const currentUsername = computed(() => {
  const user = authService.getUser()
  return user?.displayName || user?.username || '未登录'
})

const isAdmin = computed(() => authService.isAdmin())

const isCollapsed = ref(false)
const currentRoute = computed(() => route.path)
const currentMeta = computed(() => route.meta as { title?: string })

const selectedMonth = ref(uiStore.globalYearMonth)
const globalSearch = ref(uiStore.globalSearchKeyword)

watch(() => uiStore.globalYearMonth, (val) => {
  selectedMonth.value = val
})

watch(() => uiStore.globalSearchKeyword, (val) => {
  globalSearch.value = val
})

function onMonthChange(val: string) {
  uiStore.setGlobalYearMonth(val)
}

function onGlobalSearch(val: string) {
  uiStore.setGlobalSearchKeyword(val)
}

async function onGlobalRefresh() {
  await uiStore.refreshAll(roomStore, billStore, tenantStore, paymentService)
}

// 监听窗口大小变化
try {
  appWindow.onResized(() => {
    checkMaximized()
  })
} catch {
  // 非 Tauri 环境忽略
}

checkMaximized()

/** 键盘快捷键：Ctrl+Shift+D 打开CSS诊断工具 */
function handleKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.shiftKey && e.key === 'D') {
    e.preventDefault()
    showDiagnostic.value = !showDiagnostic.value
  }
}

onMounted(() => {
  // 初始化系统配置
  systemStore.loadConfigs()
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})
</script>

<style lang="scss" scoped>
.layout-container {
  height: 100vh;
}

.sidebar {
  background-color: var(--sidebar-bg);
  transition: width 0.3s cubic-bezier(0.645, 0.045, 0.355, 1);
  overflow: hidden;

  .logo {
    height: 112px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    font-weight: 600;
    color: var(--text-primary);
    background-color: var(--sidebar-logo-bg);
    white-space: nowrap;
    -webkit-app-region: drag;
    gap: 8px;

    .logo-icon {
      font-size: 28px;
      color: #000000;
    }
  }

  /* 深色模式 Logo：浅色字 + 白色图标 */
  [data-theme="dark"] & {
    .logo {
      color: #e6edf3 !important;

      .logo-icon {
        color: #ffffff !important;
      }
    }
  }

  .el-menu {
    border-right: none;
    height: calc(100% - 112px);

    .el-menu-item {
      font-size: 16px;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 5px;

      .el-icon {
        font-size: 16px;
        margin-right: 0;
      }
    }
  }
}

.header {
  background: var(--header-bg);
  border-bottom: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  box-shadow: var(--header-shadow);
  user-select: none;
  -webkit-app-region: drag;

  .header-left {
    display: flex;
    align-items: center;
    gap: 16px;

    .collapse-btn {
      font-size: 20px;
      cursor: pointer;
      color: var(--text-regular);
      -webkit-app-region: no-drag;
      &:hover { color: var(--color-primary); }
    }
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
    -webkit-app-region: no-drag;

    .month-picker {
      width: 160px;
    }

    .user-info {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 0 12px;
      border-left: 1px solid var(--border-color);
      margin-left: 8px;

      .username {
        font-size: 13px;
        color: var(--text-secondary);
      }
    }
  }

  .refresh-icon {
    transition: transform 0.3s;
    &.is-loading {
      animation: spin 0.8s linear infinite;
    }
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

// 窗口控制按钮
.window-controls {
  display: flex;
  align-items: center;
  margin-left: 8px;
}

.window-btn {
  width: 36px;
  height: 36px;
  border: none;
  background: var(--window-btn-bg);
  color: var(--window-btn-color);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: var(--window-btn-hover-bg);
    color: var(--window-btn-hover-color);
  }

  &.close:hover {
    background: var(--window-close-hover-bg);
    color: var(--window-close-hover-color);
  }

  svg {
    width: 12px;
    height: 12px;
  }
}

.main-content {
  background-color: var(--main-bg);
  padding: 0;
  overflow-y: auto;
}
</style>
