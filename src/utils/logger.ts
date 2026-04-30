/**
 * 前端日志工具
 *
 * 将关键操作日志写入 Tauri 本地文件，便于排查问题
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error'

interface LogEntry {
  timestamp: string
  level: LogLevel
  message: string
  data?: unknown
}

class Logger {
  private logs: LogEntry[] = []
  private maxMemoryLogs = 100
  private writeEnabled = false

  /**
   * 初始化日志系统（应用启动时调用）
   */
  async init(): Promise<void> {
    try {
      const { exists, mkdir } = await import('@tauri-apps/plugin-fs')
      const { appDataDir, join } = await import('@tauri-apps/api/path')

      const logDir = await appDataDir()
      const logPath = await join(logDir, 'logs')

      if (!await exists(logPath)) {
        await mkdir(logPath, { recursive: true })
      }

      this.writeEnabled = true
      this.info('Logger initialized', { logPath })
    } catch (err) {
      console.warn('[Logger] 初始化失败，仅使用内存日志:', err)
    }
  }

  /**
   * 格式化日志条目
   */
  private formatEntry(entry: LogEntry): string {
    const dataStr = entry.data ? ` | ${JSON.stringify(entry.data)}` : ''
    return `[${entry.timestamp}] [${entry.level.toUpperCase()}] ${entry.message}${dataStr}`
  }

  /**
   * 获取当前时间戳
   */
  private getTimestamp(): string {
    return new Date().toISOString().replace('T', ' ').slice(0, 19)
  }

  /**
   * 添加日志条目
   */
  private addLog(level: LogLevel, message: string, data?: unknown): void {
    const entry: LogEntry = {
      timestamp: this.getTimestamp(),
      level,
      message,
      data,
    }

    // 内存日志（限制数量）
    this.logs.push(entry)
    if (this.logs.length > this.maxMemoryLogs) {
      this.logs.shift()
    }

    // 控制台输出
    const consoleMethod = level === 'debug' ? 'debug' : level
    console[consoleMethod](`[${entry.timestamp}] [${level.toUpperCase()}]`, message, data ?? '')
  }

  /**
   * 写入日志文件（异步，不阻塞主流程）
   * 注意：plugin-fs v2 没有 appendFile，使用 read/write 模拟追加
   */
  private async writeToFile(entry: LogEntry): Promise<void> {
    if (!this.writeEnabled) return

    try {
      const { readTextFile, writeTextFile, exists } = await import('@tauri-apps/plugin-fs')
      const { appDataDir, join } = await import('@tauri-apps/api/path')

      const date = new Date().toISOString().slice(0, 10)
      const logDir = await appDataDir()
      const logFile = await join(logDir, 'logs', `app-${date}.log`)

      const content = this.formatEntry(entry) + '\n'
      let existing = ''
      if (await exists(logFile)) {
        existing = await readTextFile(logFile)
      }
      await writeTextFile(logFile, existing + content)
    } catch (err) {
      console.warn('[Logger] 写入日志文件失败:', err)
    }
  }

  debug(message: string, data?: unknown): void {
    this.addLog('debug', message, data)
  }

  info(message: string, data?: unknown): void {
    const entry: LogEntry = {
      timestamp: this.getTimestamp(),
      level: 'info',
      message,
      data,
    }
    this.addLog('info', message, data)
    this.writeToFile(entry)
  }

  warn(message: string, data?: unknown): void {
    const entry: LogEntry = {
      timestamp: this.getTimestamp(),
      level: 'warn',
      message,
      data,
    }
    this.addLog('warn', message, data)
    this.writeToFile(entry)
  }

  error(message: string, data?: unknown): void {
    const entry: LogEntry = {
      timestamp: this.getTimestamp(),
      level: 'error',
      message,
      data,
    }
    this.addLog('error', message, data)
    this.writeToFile(entry)
  }

  /**
   * 获取内存中的日志（用于调试）
   */
  getLogs(): LogEntry[] {
    return [...this.logs]
  }

  /**
   * 清空内存日志
   */
  clear(): void {
    this.logs = []
  }
}

// 单例导出
export const logger = new Logger()
