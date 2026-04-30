/**
 * 全局事件总线
 * 基于 src/utils/signal.ts 的发布/订阅模式
 *
 * 使用场景:
 * - RoomList 双击卡片 → 触发 BillPrintDrawer 打开
 * - 跨路由/跨层级的组件通信
 * - 替代 Vue bus/EventEmitter 更轻量的方案
 *
 * @example
 * // 监听（BillPrintDrawer）
 * import { roomEvents } from '@/events'
 * const unsub = roomEvents.openBillPrint.subscribe(({ roomId, billId }) => { ... })
 * onUnmounted(unsub)
 *
 * // 触发（RoomList）
 * roomEvents.openBillPrint.emit({ roomId: 1, billId: 100 })
 */

import { createSignal } from '@/utils/signal'

// ---------------------------------------------------------------------------
// 事件类型定义
// ---------------------------------------------------------------------------

export interface OpenBillPrintEvent {
  roomId: number
  billId?: number
  room: unknown // RoomResponse
}

export interface RoomStatusChangeEvent {
  roomId: number
  oldStatus: string
  newStatus: string
}

// ---------------------------------------------------------------------------
// 事件总线
// ---------------------------------------------------------------------------

export const roomEvents = {
  /**
   * 打开收费通知单
   * 触发方: RoomList（双击卡片）
   * 监听方: BillPrintDrawer / 任何需要响应此事件的组件
   */
  openBillPrint: createSignal<[OpenBillPrintEvent]>(),

  /**
   * 房间状态变更
   * 触发方: RoomList
   * 监听方: Dashboard、Reminders 等需要同步状态的组件
   */
  roomStatusChanged: createSignal<[RoomStatusChangeEvent]>(),

  /**
   * 账单已支付
   * 触发方: BillDetail、PaymentList
   * 监听方: RoomList（刷新房间状态）、Dashboard（刷新统计）
   */
  billPaid: createSignal<[{ billId: number; roomId: number }]>(),
} as const
