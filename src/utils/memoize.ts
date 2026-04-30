/**
 * Memoization 缓存工具
 * 来源: Claude Code cli/src/utils/memoize.ts
 *
 * 特性:
 * - memoizeWithTTL: 基于时间的缓存过期
 * - memoizeWithLRU: 基于数量的 LRU 淘汰
 *
 * 使用场景:
 * - API 请求结果缓存（如 getBillDetail, getAllConfigs）
 * - expensive computed（如 roomStore.filteredRooms 的复杂过滤）
 * - 避免重复的计算（如 formatMoneyInt 重复调用）
 *
 * @example
 * // TTL 缓存：账单详情 5 分钟不过期
 * const getBillDetailCached = memoizeWithTTL(
 *   (id: number) => billService.getBillDetail(id),
 *   5 * 60 * 1000
 * )
 *
 * // LRU 缓存：最近 50 条账单
 * const getRoomCached = memoizeWithLRU(
 *   (id: number) => roomService.getRoom(id),
 *   new Map(),
 *   50
 * )
 */

// ---------------------------------------------------------------------------
// TTL-based memoization
// ---------------------------------------------------------------------------

type AsyncFn<T extends unknown[], R> = (...args: T) => Promise<R>
type SyncFn<T extends unknown[], R> = (...args: T) => R

interface CacheEntry<R> {
  value: R
  expiresAt: number
}

export function memoizeWithTTL<T extends unknown[], R>(
  fn: AsyncFn<T, R>,
  cacheLifetimeMs: number,
): AsyncFn<T, R> {
  const cache = new Map<string, CacheEntry<R>>()

  return async (...args: T): Promise<R> => {
    const key = JSON.stringify(args)
    const now = Date.now()
    const entry = cache.get(key)

    if (entry && entry.expiresAt > now) {
      return entry.value
    }

    const value = await fn(...args)
    cache.set(key, { value, expiresAt: now + cacheLifetimeMs })
    return value
  }
}

export function memoizeWithTTLSync<T extends unknown[], R>(
  fn: SyncFn<T, R>,
  cacheLifetimeMs: number,
): SyncFn<T, R> {
  const cache = new Map<string, CacheEntry<R>>()

  return (...args: T): R => {
    const key = JSON.stringify(args)
    const now = Date.now()
    const entry = cache.get(key)

    if (entry && entry.expiresAt > now) {
      return entry.value
    }

    const value = fn(...args)
    cache.set(key, { value, expiresAt: now + cacheLifetimeMs })
    return value
  }
}

// ---------------------------------------------------------------------------
// LRU memoization
// ---------------------------------------------------------------------------

export function memoizeWithLRU<T extends unknown[], R>(
  fn: SyncFn<T, R>,
  cache: Map<string, R>,
  maxCacheSize: number,
): SyncFn<T, R> {
  return (...args: T): R => {
    const key = JSON.stringify(args)

    if (cache.has(key)) {
      // 移到最末（最新使用）
      const value = cache.get(key)!
      cache.delete(key)
      cache.set(key, value)
      return value
    }

    if (cache.size >= maxCacheSize) {
      // 删除最老的（Map 的插入顺序即 LRU 顺序）
      const firstKey = cache.keys().next().value
      if (firstKey !== undefined) cache.delete(firstKey)
    }

    const value = fn(...args)
    cache.set(key, value)
    return value
  }
}

export function memoizeWithLRUAsync<T extends unknown[], R>(
  fn: AsyncFn<T, R>,
  cache: Map<string, R>,
  maxCacheSize: number,
): AsyncFn<T, R> {
  return async (...args: T): Promise<R> => {
    const key = JSON.stringify(args)

    if (cache.has(key)) {
      const value = cache.get(key)!
      cache.delete(key)
      cache.set(key, value)
      return value
    }

    if (cache.size >= maxCacheSize) {
      const firstKey = cache.keys().next().value
      if (firstKey !== undefined) cache.delete(firstKey)
    }

    const value = await fn(...args)
    cache.set(key, value)
    return value
  }
}
