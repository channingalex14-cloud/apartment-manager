/**
 * 系统配置 Store - 管理全局配置项（单价、公寓名称等）
 * 适配 V2 的 configService API（基于 key 字符串而非 ID）
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { configService } from '@/services/config.service'
import type { SystemConfig } from '@/types/config'
import { toYuanNumber } from '@/utils/money'

// 配置 key 常量（与数据库 schema 一致，使用中文 key）
// 注意：数据库中配置值以"分"为单位存储（如 水费单价=600 表示 6 元/吨）
export const CONFIG_KEYS = {
  WATER_PRICE: '水费单价',
  ELEC_PRICE: '电费单价',
  MGMT_PRICE: '管理费单价',
  APT_NAME: '公寓名称',
  BUILDING: '楼栋地址',
  BANK_ACCOUNT_NAME: '银行户名',
  BANK_ACCOUNT: '银行账号',
  BANK_NAME: '开户行',
} as const

export const useSystemStore = defineStore('system', () => {
  const configs = ref<SystemConfig[]>([])
  const loading = ref(false)
  const initialized = ref(false)

  /** 根据 key 获取配置值 */
  function getConfigValue(key: string): string {
    return configs.value.find(c => c.config_key === key)?.config_value || ''
  }

  /** 水费单价（分转元） */
  const waterPrice = computed(() => {
    const raw = getConfigValue(CONFIG_KEYS.WATER_PRICE)
    if (!raw) return 6.00
    return toYuanNumber(parseInt(raw, 10))
  })

  /** 电费单价（分转元） */
  const elecPrice = computed(() => {
    const raw = getConfigValue(CONFIG_KEYS.ELEC_PRICE)
    if (!raw) return 0.73
    return toYuanNumber(parseInt(raw, 10))
  })

  /** 管理费单价（分转元） */
  const mgmtPrice = computed(() => {
    const raw = getConfigValue(CONFIG_KEYS.MGMT_PRICE)
    if (!raw) return 0.57
    return toYuanNumber(parseInt(raw, 10))
  })

  /** 公寓名称 */
  const aptName = computed(() => getConfigValue(CONFIG_KEYS.APT_NAME) || '新逸公寓')

  /** 楼栋 */
  const building = computed(() => getConfigValue(CONFIG_KEYS.BUILDING) || '58栋')

  /** 银行信息 */
  const bankInfo = computed(() => ({
    户名: getConfigValue(CONFIG_KEYS.BANK_ACCOUNT_NAME) || '陈华康',
    账号: getConfigValue(CONFIG_KEYS.BANK_ACCOUNT) || '6226 2206 3475 5002',
    开户行: getConfigValue(CONFIG_KEYS.BANK_NAME) || '民生银行景田支行',
  }))

  /** 价格配置合集（供费用计算用） */
  const prices = computed(() => ({
    waterPrice: waterPrice.value,
    elecPrice: elecPrice.value,
    mgmtPrice: mgmtPrice.value,
  }))

  async function loadConfigs() {
    if (loading.value) return
    loading.value = true
    try {
      configs.value = await configService.getAll()
      initialized.value = true
    } finally {
      loading.value = false
    }
  }

  async function saveConfig(key: string, value: string) {
    await configService.set(key, value)
    const config = configs.value.find(c => c.config_key === key)
    if (config) {
      config.config_value = value
    }
  }

  async function saveConfigs(configsToSave: Array<{ key: string; value: string }>) {
    for (const { key, value } of configsToSave) {
      await configService.set(key, value)
      const config = configs.value.find(c => c.config_key === key)
      if (config) {
        config.config_value = value
      }
    }
  }

  return {
    configs,
    loading,
    initialized,
    waterPrice,
    elecPrice,
    mgmtPrice,
    aptName,
    building,
    bankInfo,
    prices,
    loadConfigs,
    saveConfig,
    saveConfigs,
    getConfigValue,
  }
})
