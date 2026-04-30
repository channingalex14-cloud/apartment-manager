/**
 * 错误处理工具
 * 来源: Claude Code cli/src/utils/errors.ts
 *
 * 特性:
 * - 错误类型分类（中止/网络/文件系统等）
 * - 统一错误消息提取
 * - 错误转换为标准 Error 对象
 *
 * 使用场景:
 * - Tauri API 调用的错误处理
 * - 网络请求的细粒度错误分类
 * - 在 UI 层统一展示用户友好的错误信息
 *
 * @example
 * try {
 *   await billService.getBillDetail(id)
 * } catch (e) {
 *   if (isENOENT(e)) { ElMessage.error('账单不存在') }
 *   else if (isAbortError(e)) { /* 用户取消 *\/ }
 *   else { ElMessage.error(errorMessage(e)) }
 * }
 */

// ---------------------------------------------------------------------------
// 自定义错误类
// ---------------------------------------------------------------------------

export class ClaudeError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ClaudeError'
  }
}

export class AbortError extends Error {
  constructor(message = '操作已取消') {
    super(message)
    this.name = 'AbortError'
  }
}

export class NetworkError extends Error {
  constructor(
    message: string,
    public status?: number,
  ) {
    super(message)
    this.name = 'NetworkError'
  }
}

export class ValidationError extends Error {
  constructor(
    message: string,
    public field?: string,
  ) {
    super(message)
    this.name = 'ValidationError'
  }
}

// ---------------------------------------------------------------------------
// 错误判断工具
// ---------------------------------------------------------------------------

export function isAbortError(e: unknown): boolean {
  if (e instanceof AbortError) return true
  if (e instanceof Error && e.name === 'AbortError') return true
  // DOM abort 或 axios 取消
  if (e && typeof e === 'object' && 'name' in e) {
    const name = (e as { name: string }).name
    if (name === 'CanceledError' || name === 'AbortError') return true
  }
  return false
}

export function isNetworkError(e: unknown): boolean {
  if (e instanceof NetworkError) return true
  if (e instanceof Error && e.name === 'NetworkError') return true
  return false
}

export function isENOENT(e: unknown): boolean {
  if (e instanceof Error) {
    const err = e as Error & { code?: string; errno?: number }
    // Node.js ENOENT
    if (err.code === 'ENOENT') return true
    // Windows 文件未找到
    if (err.errno === -4058) return true
  }
  return false
}

export function isFsInaccessible(e: unknown): boolean {
  if (e instanceof Error) {
    const err = e as Error & { code?: string; errno?: number }
    if (err.code === 'EACCES' || err.code === 'EPERM') return true
    // Windows 拒绝访问
    if (err.errno === -13) return true
  }
  return false
}

export function isValidationError(e: unknown): boolean {
  return e instanceof ValidationError
}

// ---------------------------------------------------------------------------
// 错误转换与消息提取
// ---------------------------------------------------------------------------

export function toError(e: unknown): Error {
  if (e instanceof Error) return e
  if (typeof e === 'string') return new Error(e)
  return new Error(String(e))
}

export function errorMessage(e: unknown): string {
  if (isAbortError(e)) return '操作已取消'
  if (e instanceof Error) return e.message || '未知错误'
  if (typeof e === 'string') return e
  return '发生了未知错误'
}

/**
 * 从错误中提取 HTTP 状态码（用于网络请求错误）
 */
export function getHttpStatus(e: unknown): number | undefined {
  if (e && typeof e === 'object') {
    const err = e as { response?: { status?: number }; status?: number }
    if (err.response?.status) return err.response.status
    if (err.status) return err.status
  }
  return undefined
}

/**
 * 判断是否是"账单不存在"类型的错误
 */
export function isNotFoundError(e: unknown): boolean {
  const status = getHttpStatus(e)
  if (status === 404) return true
  if (e instanceof Error && e.message.includes('不存在')) return true
  return false
}

/**
 * 分类 Tauri 命令执行错误
 */
export function classifyTauriError(e: unknown): {
  kind: 'network' | 'not_found' | 'permission' | 'unknown'
  message: string
} {
  if (isENOENT(e)) return { kind: 'not_found', message: '文件或数据不存在' }
  if (isFsInaccessible(e)) return { kind: 'permission', message: '无访问权限' }
  if (isNotFoundError(e)) return { kind: 'not_found', message: '资源不存在' }

  const status = getHttpStatus(e)
  if (status === 401 || status === 403) {
    return { kind: 'permission', message: '无权限执行此操作' }
  }
  if (status === 500) {
    return { kind: 'unknown', message: '服务器内部错误，请稍后重试' }
  }

  return { kind: 'unknown', message: errorMessage(e) }
}
