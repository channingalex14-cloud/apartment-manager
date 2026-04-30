/**
 * JSON 解析工具
 * 来源: Claude Code cli/src/utils/json.ts
 *
 * 特性:
 * - safeParseJSON: 带错误日志的安全解析，失败返回 null
 * - parseJSONL: 解析 JSONL（每行一个 JSON）格式
 * - JSONC 支持: 保留注释的 JSON 解析（用于 settings.json 等）
 *
 * 使用场景:
 * - 解析 Tauri 配置文件
 * - 解析 bill-assets 目录下的静态资源
 * - 日志文件的批量解析
 *
 * @example
 * const config = safeParseJSON(rawString)
 * if (!config) { ElMessage.error('配置解析失败') }
 *
 * const entries = parseJSONL(logContent)
 */

const JSON_CACHE = new Map<string, { value: unknown; ts: number }>()
const JSON_CACHE_MAX_AGE_MS = 5000 // 5秒短时缓存

// ---------------------------------------------------------------------------
// 安全 JSON 解析
// ---------------------------------------------------------------------------

/**
 * 安全解析 JSON，失败时记录错误并返回 null
 * 内部有 5 秒缓存避免重复解析同一个字符串
 */
export function safeParseJSON<T = unknown>(
  json: string | null | undefined,
  shouldLogError = false,
): T | null {
  if (!json) return null

  const cached = JSON_CACHE.get(json)
  if (cached && Date.now() - cached.ts < JSON_CACHE_MAX_AGE_MS) {
    return cached.value as T
  }

  try {
    const value = JSON.parse(json) as T
    JSON_CACHE.set(json, { value, ts: Date.now() })
    return value
  } catch (err) {
    if (shouldLogError) {
      console.error('[safeParseJSON] parse error:', err, 'input:', json.slice(0, 200))
    }
    return null
  }
}

/**
 * 解析 JSONC（带注释的 JSON），支持单行和多行注释
 */
export function parseJSONC<T = unknown>(content: string): T | null {
  // 去除注释
  const cleaned = content
    .replace(/\/\*[\s\S]*?\*\//g, '') // /* ... */
    .replace(/\/\/.*$/gm, '') // // ...
    .trim()

  if (!cleaned || cleaned === '{' || cleaned === '[') {
    return null
  }

  return safeParseJSON<T>(cleaned, true)
}

/**
 * 解析 JSONL 格式（每行一个 JSON），返回数组
 */
export function parseJSONL<T>(data: string): T[] {
  const lines = data.split('\n').filter((l) => l.trim() !== '')
  const results: T[] = []

  for (const line of lines) {
    const parsed = safeParseJSON<T>(line)
    if (parsed !== null) {
      results.push(parsed)
    }
  }

  return results
}

// ---------------------------------------------------------------------------
// JSON 序列化（带错误处理）
// ---------------------------------------------------------------------------

/**
 * 安全序列化 JSON，失败返回空对象字面量
 */
export function safeStringify(value: unknown, space?: number): string {
  try {
    return JSON.stringify(value, null, space)
  } catch (err) {
    console.error('[safeStringify] error:', err)
    return '{}'
  }
}

/**
 * 格式化 Zod 验证错误为可读字符串
 * 兼容 Zod v3 和 v4
 */
export function formatZodError(error: unknown): string {
  if (!error || typeof error !== 'object') return String(error)

  // Zod v3: error.issues, Zod v4: error.errors
  const issues = (error as { issues?: unknown[]; errors?: unknown[] }).issues ?? (error as { errors?: unknown[] }).errors
  if (!Array.isArray(issues)) return String(error)

  return issues
    .map((issue: unknown) => {
      const zodIssue = issue as { path?: (string | number)[]; message?: string; received?: unknown }
      const path = zodIssue.path?.join('.') || 'unknown field'
      const received = zodIssue.received ? `（收到: ${zodIssue.received}）` : ''
      return `${path}: ${zodIssue.message ?? '未知错误'}${received}`
    })
    .join('\n')
}
