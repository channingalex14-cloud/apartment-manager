/**
 * 延迟 Schema 构造工具
 * 来源: Claude Code cli/src/utils/lazySchema.ts
 *
 * 特性:
 * - 首次调用时才构造 Zod schema，避免模块初始化时的循环依赖
 * - 之后调用直接返回缓存结果
 *
 * 使用场景:
 * - 当多个 Zod schema 互相引用时（RoomSchema ↔ TenantSchema）
 * - 避免顶层 import 时的循环依赖问题
 * - 大型表单验证 schema 的懒加载
 *
 * @example
 * // 两个 schema 互相引用，不会循环初始化
 * export const RoomSchema = lazySchema(() => z.object({ tenant: TenantSchema() }))
 * export const TenantSchema = lazySchema(() => z.object({ room: RoomSchema() }))
 */

export function lazySchema<T>(factory: () => T): () => T {
  let cached: T | undefined
  return () => (cached ??= factory())
}
