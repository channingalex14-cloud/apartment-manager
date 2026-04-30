/**
 * UI 状态 Store - 管理全局 UI 状态（当前月份、loading、dialog 等）
 * 适配 V2 的 dayjs 工具
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import dayjs from 'dayjs'

/** 获取当前年月 YYYY-MM */
function getCurrentMonth(): string {
  return dayjs().format('YYYY-MM')
}

export const useUIStore = defineStore('ui', () => {
  /** 全局当前年月（影响所有页面的数据范围） */
  const globalYearMonth = ref(getCurrentMonth())

  /** 全局 loading */
  const appLoading = ref(false)

  /** 数据库是否已初始化 */
  const dbInitialized = ref(false)

  /** 全局搜索关键词（房号/姓名） */
  const globalSearchKeyword = ref("")

  /** 格式化后的年月显示 */
  const displayYearMonth = computed(() => {
    const [y, m] = globalYearMonth.value.split('-')
    return `${y}年${parseInt(m || '1')}月`
  })

  function setGlobalYearMonth(ym: string) {
    globalYearMonth.value = ym
  }

  function setAppLoading(val: boolean) {
    appLoading.value = val
  }

  function setDbInitialized(val: boolean) {
    dbInitialized.value = val
  }

  function setGlobalSearchKeyword(val: string) {
    globalSearchKeyword.value = val
  }

  /** 全局刷新所有数据 */
  async function refreshAll(
    roomStore: { fetchRooms: () => Promise<unknown> },
    billStore: { fetchBills: () => Promise<unknown> },
    tenantStore: { fetchTenants: () => Promise<unknown> },
    paymentService: { list: () => Promise<unknown> },
  ) {
    appLoading.value = true
    try {
      await Promise.all([
        roomStore.fetchRooms(),
        billStore.fetchBills(),
        tenantStore.fetchTenants(),
        paymentService.list(),
      ])
    } finally {
      appLoading.value = false
    }
  }

  return {
    globalYearMonth,
    appLoading,
    dbInitialized,
    displayYearMonth,
    globalSearchKeyword,
    setGlobalYearMonth,
    setAppLoading,
    setDbInitialized,
    setGlobalSearchKeyword,
    refreshAll,
  }
})
