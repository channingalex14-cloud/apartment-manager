/**
 * 重试工具 - 指数退避 + Jitter
 * 来源: Claude Code cli/src/services/api/withRetry.ts
 *
 * 特性:
 * - 指数退避：每次重试等待时间翻倍
 * - Jitter：避免多客户端同时重试造成惊群效应
 * - 可配置最大重试次数
 * - 可中止：通过 AbortController
 *
 * 使用场景:
 * - Tauri 命令调用（网络调用）
 * - 账单查询、房间数据同步
 * - 支付确认等关键操作
 *
 * @example
 * // 账单查询失败自动重试最多 3 次
 * const bills = await withRetry(
 *   () => billService.queryBills(params),
 *   { maxRetries: 3, baseDelayMs: 500 }
 * )
 */

export interface RetryOptions {
  maxRetries?: number
  baseDelayMs?: number
  maxDelayMs?: number
  signal?: AbortSignal
  /** 标签，用于日志 */
  label?: string
}

/**
 * 带指数退避和随机 jitter 的重试包装
 *
 * @param fn 要执行的异步操作
 * @param options 重试配置
 * @returns 如果所有重试都失败，抛出最后一个错误
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  options: RetryOptions = {},
): Promise<T> {
  const {
    maxRetries = 3,
    baseDelayMs = 500,
    maxDelayMs = 32000,
    signal,
    label = 'operation',
  } = options

  let lastError: unknown

  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    try {
      // 检查是否已中止
      if (signal?.aborted) {
        throw new AbortError('操作已取消')
      }
      return await fn()
    } catch (err) {
      lastError = err

      // 不重试中止错误
      if (err instanceof DOMException && err.name === 'AbortError') {
        throw err
      }

      // 最后一次尝试失败，直接抛出
      if (attempt > maxRetries) {
        break
      }

      // 计算退避时间：base * 2^(attempt-1)，加上随机 jitter（0~25%）
      const baseDelay = Math.min(baseDelayMs * Math.pow(2, attempt - 1), maxDelayMs)
      const jitter = Math.random() * 0.25 * baseDelay
      const delayMs = Math.round(baseDelay + jitter)

      console.warn(
        `[withRetry] ${label} 失败（第 ${attempt} 次），${delayMs}ms 后重试...`,
        err instanceof Error ? err.message : String(err),
      )

      await sleep(delayMs, signal)
    }
  }

  throw lastError
}

/**
 * 带取消信号的 sleep
 */
export function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(resolve, ms)
    signal?.addEventListener('abort', () => {
      clearTimeout(timeout)
      reject(new DOMException('Aborted', 'AbortError'))
    })
  })
}

/**
 * 自定义中止错误类（用于类型区分）
 */
export class AbortError extends Error {
  name = 'AbortError'
  constructor(message = '操作已取消') {
    super(message)
  }
}

// ---------------------------------------------------------------------------
// 专用版本：前端 Promise（无 AbortSignal）
// ---------------------------------------------------------------------------

/**
 * 简化版重试（用于 Vue/前端，无 AbortSignal）
 */
export async function retryPromise<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  baseDelayMs = 500,
): Promise<T> {
  let lastError: unknown

  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    try {
      return await fn()
    } catch (err) {
      lastError = err
      if (attempt > maxRetries) break

      const delayMs = Math.round(
        Math.min(baseDelayMs * Math.pow(2, attempt - 1), 32000) * (1 + Math.random() * 0.25),
      )
      await new Promise((r) => setTimeout(r, delayMs))
    }
  }

  throw lastError
}
