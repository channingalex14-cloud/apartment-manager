/**
 * 路径工具
 * 来源: Claude Code cli/src/utils/path.ts
 *
 * 特性:
 * - expandPath: 处理 ~、相对路径、绝对路径，跨平台兼容
 * - containsPathTraversal: 检测路径遍历攻击（..）
 * - toRelativePath: 转换为相对路径
 *
 * 使用场景:
 * - Tauri 处理文件路径（asset 路径、导出路径）
 * - 处理用户上传/导入的文件路径
 * - 拼接 bill-assets 等静态资源目录
 *
 * @example
 * const assetPath = await expandPath('~/Documents/bills')
 * const safe = containsPathTraversal(userInput) // 防注入
 */

import { join, resolve, normalize, isAbsolute, dirname, appDataDir } from '@tauri-apps/api/path'

// ---------------------------------------------------------------------------
// 路径展开
// ---------------------------------------------------------------------------

/**
 * 展开路径中的 ~（用户目录）、环境变量、相对路径
 * 返回绝对路径
 */
export async function expandPath(
  inputPath: string,
  baseDir?: string,
): Promise<string> {
  let resolved = inputPath.trim()

  // 展开 ~
  if (resolved.startsWith('~/') || resolved === '~') {
    const home = await getHomeDir()
    resolved = resolved.replace(/^~\/?/, home + '/')
  }

  // 展开环境变量 ${VAR}
  resolved = resolved.replace(/\$\{([^}]+)\}/g, (_, varName) => {
    return process.env[varName] || ''
  })
  resolved = resolved.replace(/\$(\w+)/g, (_, varName) => {
    return process.env[varName] || ''
  })

  // 转为绝对路径
  if (!(await isAbsolute(resolved))) {
    const base = baseDir || await getCwd()
    resolved = await join(base, resolved)
  }

  return await normalize(resolved)
}

/**
 * 同步版本（仅处理 ~ 展开）
 */
export function expandPathSync(inputPath: string): string {
  let resolved = inputPath.trim()

  if (resolved.startsWith('~/') || resolved === '~') {
    const home = getHomeDirSync()
    resolved = resolved.replace(/^~\/?/, home + '/')
  }

  return resolved
}

// ---------------------------------------------------------------------------
// 路径遍历检测
// ---------------------------------------------------------------------------

/**
 * 检测路径中是否包含 .. 路径遍历
 * 防止用户输入被用于路径穿透攻击
 */
export async function containsPathTraversal(path: string): Promise<boolean> {
  const normalized = await normalize(path)
  // 包含 .. 或以 .. 开头
  return (
    normalized.includes('..') ||
    /^\.\./.test(path) ||
    /\/\.\./.test(path) ||
    /^\.\.[\\/]/.test(path)
  )
}

// ---------------------------------------------------------------------------
// 相对路径
// ---------------------------------------------------------------------------

/**
 * 将绝对路径转为相对于 cwd 的路径
 */
export async function toRelativePath(absolutePath: string): Promise<string> {
  const cwd = await getCwd()
  return await resolve(cwd, '..', absolutePath)
}

// ---------------------------------------------------------------------------
// 路径拼接
// ---------------------------------------------------------------------------

/**
 * 安全拼接 bill-assets 目录下的资源路径
 */
export async function joinBillAsset(...parts: string[]): Promise<string> {
  const joined = await join('/bill-assets', ...parts)
  return joined.replace(/\\/g, '/') // Windows 下保持正斜杠
}

// ---------------------------------------------------------------------------
// 路径安全检测
// 来源: Claude Code cli/src/utils/permissions/filesystem.ts
// ---------------------------------------------------------------------------

export type Platform = 'windows' | 'macos' | 'linux' | 'unknown'

export function getPlatform(): Platform {
  const raw = navigator.userAgent.toLowerCase()
  if (raw.includes('win')) return 'windows'
  if (raw.includes('mac')) return 'macos'
  if (raw.includes('linux')) return 'linux'
  return 'unknown'
}

/**
 * Windows 可疑路径模式检测
 * 防止 NTFS 流、8.3 短名、长路径前缀等攻击向量
 */
export function hasSuspiciousWindowsPathPattern(path: string): boolean {
  // NTFS 交替数据流: file.txt::$DATA
  if (getPlatform() === 'windows' && path.includes(':', 2)) return true
  // 8.3 短名: GIT~1, CLAUDE~1
  if (/~\d/.test(path)) return true
  // 长路径前缀: \\?\C:\...
  if (path.startsWith('\\\\?\\') || path.startsWith('\\\\.\\')) return true
  // DOS 设备名: .git.CON, settings.json.PRN
  if (/\.(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/i.test(path)) return true
  // 三个或更多连续点: .../file.txt
  if (/(^|\/|\\)\.{3,}(\/|\\|$)/.test(path)) return true
  return false
}

/**
 * 检查路径是否安全（无路径遍历、无 Windows 可疑模式）
 * 用于验证用户输入的路径（导出路径、文件名等）
 *
 * @returns { safe: true } 或 { safe: false, reason: string }
 */
export async function checkPathSafety(
  path: string,
): Promise<{ safe: true } | { safe: false; reason: string }> {
  // 检测路径遍历
  if (await containsPathTraversal(path)) {
    return { safe: false, reason: '路径包含 .. 遍历序列' }
  }
  // 检测 Windows 可疑模式
  if (hasSuspiciousWindowsPathPattern(path)) {
    return { safe: false, reason: '路径包含 Windows 可疑模式（NTFS 流、8.3 名称等）' }
  }
  return { safe: true }
}

/**
 * 安全地验证并展开用户提供的路径
 * 展开 ~ 和环境变量，检测路径遍历攻击
 */
export async function safeExpandPath(
  inputPath: string,
  baseDir?: string,
): Promise<{ path: string; safe: boolean; reason?: string }> {
  // 先展开
  const expanded = await expandPath(inputPath, baseDir)
  // 再安全检测
  const safety = await checkPathSafety(expanded)
  return {
    path: expanded,
    safe: safety.safe,
    reason: safety.safe ? undefined : (safety as { safe: false; reason: string }).reason,
  }
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

async function getHomeDir(): Promise<string> {
  // Tauri 环境
  try {
    const { homeDir } = await import('@tauri-apps/api/path')
    return await homeDir()
  } catch {
    return 'C:/Users/' + (process.env.USERNAME || 'default')
  }
}

function getHomeDirSync(): string {
  return 'C:/Users/' + (process.env.USERNAME || 'default')
}

async function getCwd(): Promise<string> {
  try {
    // Tauri v2: use appDataDir as fallback for current directory
    // There's no direct currentDir in Tauri v2
    const dataDir = await appDataDir()
    return dataDir
  } catch {
    return process.cwd?.() || '.'
  }
}
