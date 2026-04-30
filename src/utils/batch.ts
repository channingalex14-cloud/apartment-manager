/**
 * 批量状态更新工具
 * 来源: Claude Code cli/src/services/mcp/useManageMCPConnections.ts
 *
 * 特性:
 * - 在一个时间窗口（默认 16ms）内合并多次状态更新
 * - 避免高频更新导致 Vue 组件频繁重渲染
 * - 保证最终一致性：窗口结束时一次性应用所有更新
 *
 * 使用场景:
 * - 房间列表数据批量更新（如从 Tauri 后端批量推送）
 * - 账单状态批量变化
 * - 筛选条件变化时的批量数据获取
 *
 * @example
 * // 创建批量更新器
 * const batchUpdate = createBatchUpdater(
 *   (items) => { roomStore.rooms = items },
 *   16 // 16ms 窗口
 * )
 *
 * // 快速连续调用，只触发一次渲染
 * batchUpdate([...newRooms])
 * batchUpdate([...moreRooms])  // 合并到同一批次
 */
import { onUnmounted } from 'vue'

export interface BatchUpdater<T> {
  /**
   * 添加一项到当前批次
   * 在窗口期内多次调用只会触发一次最终更新
   */
  add: (item: T) => void
  /**
   * 直接设置完整数据（替换模式）
   * 也会进入批量队列
   */
  set: (items: T) => void
  /**
   * 手动 flush 当前批次（立即执行）
   */
  flush: () => void
  /**
   * 清理，取消待处理的 flush
   */
  destroy: () => void
}

/**
 * 创建一个批量更新器
 *
 * @param apply 应用函数，窗口期结束后被调用
 * @param flushIntervalMs 批量窗口时间，默认 16ms（约一帧）
 */
export function createBatchUpdater<T>(
  apply: (items: T) => void,
  flushIntervalMs = 16,
): BatchUpdater<T> {
  let pendingItems: T | null = null
  let flushTimer: ReturnType<typeof setTimeout> | null = null
  let destroyed = false

  function scheduleFlush() {
    if (flushTimer !== null || destroyed) return
    flushTimer = setTimeout(() => {
      flushTimer = null
      if (pendingItems !== null) {
        const items = pendingItems
        pendingItems = null
        apply(items)
      }
    }, flushIntervalMs)
  }

  return {
    add(item: T) {
      if (destroyed) return
      if (pendingItems === null) {
        pendingItems = item as unknown as T
      } else {
        // merge: append item to existing
        if (Array.isArray(pendingItems) && Array.isArray(item)) {
          pendingItems = [...pendingItems, ...item] as unknown as T
        }
      }
      scheduleFlush()
    },

    set(item: T) {
      if (destroyed) return
      pendingItems = item
      scheduleFlush()
    },

    flush() {
      if (flushTimer !== null) {
        clearTimeout(flushTimer)
        flushTimer = null
      }
      if (pendingItems !== null) {
        const items = pendingItems
        pendingItems = null
        apply(items)
      }
    },

    destroy() {
      destroyed = true
      if (flushTimer !== null) {
        clearTimeout(flushTimer)
        flushTimer = null
      }
      pendingItems = null
    },
  }
}

/**
 * Vue 组合式风格的批量更新（自动在 onUnmounted 清理）
 *
 * @example
 * const batchRooms = useBatchUpdater(
 *   (rooms) => { roomStore.rooms = rooms },
 *   16
 * )
 *
 * // 快速连续设置，只渲染一次
 * batchRooms.set([...])
 * batchRooms.set([...]) // 合并
 */
export function useBatchUpdater<T>(
  apply: (items: T) => void,
  flushIntervalMs = 16,
): BatchUpdater<T> {
  const updater = createBatchUpdater(apply, flushIntervalMs)
  onUnmounted(() => updater.destroy())
  return updater
}
