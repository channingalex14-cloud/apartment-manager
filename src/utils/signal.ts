/**
 * Signal - 轻量级发布/订阅事件系统
 * 来源: Claude Code cli/src/utils/signal.ts
 *
 * 特性:
 * - 无状态存储，纯粹的事件通知
 * - 返回 unsubscribe 函数，方便在 onUnmounted 中清理
 * - 类型安全，支持任意参数类型
 *
 * 使用场景:
 * - 跨组件事件通信（如 RoomList → BillPrintDrawer）
 * - 替代 mitt/EventEmitter 更轻量
 * - 替代 Vue emit 跨层级通信
 *
 * @example
 * // 定义全局事件总线
 * export const billEvents = createSignal<{ roomId: number; billId: number }>()
 *
 * // 监听（组件 A）
 * const unsub = billEvents.subscribe((data) => { ... })
 * onUnmounted(unsub)
 *
 * // 触发（组件 B）
 * billEvents.emit(roomId, billId)
 */

export interface Signal<Args extends unknown[] = unknown[]> {
  subscribe: (listener: (...args: Args) => void) => () => void
  emit: (...args: Args) => void
  clear: () => void
}

export function createSignal<Args extends unknown[] = unknown[]>(): Signal<Args> {
  const listeners = new Set<(...args: Args) => void>()

  return {
    subscribe(listener: (...args: Args) => void): () => void {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },

    emit(...args: Args): void {
      for (const listener of listeners) {
        try {
          listener(...args)
        } catch (err) {
          console.error('[Signal] listener error:', err)
        }
      }
    },

    clear(): void {
      listeners.clear()
    },
  }
}
