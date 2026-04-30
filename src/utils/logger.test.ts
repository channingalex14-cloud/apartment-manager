import { describe, it, expect, vi, beforeEach } from 'vitest'
import { logger } from './logger'

describe('Logger', () => {
  beforeEach(() => {
    logger.clear()
    vi.clearAllMocks()
    // 抑制控制台输出
    vi.spyOn(console, 'debug').mockImplementation(() => {})
    vi.spyOn(console, 'info').mockImplementation(() => {})
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  describe('memory logging', () => {
    it('should store logs in memory', () => {
      logger.info('test message')
      const logs = logger.getLogs()
      expect(logs).toHaveLength(1)
      expect(logs[0]!.level).toBe('info')
      expect(logs[0]!.message).toBe('test message')
    })

    it('should store log data', () => {
      logger.error('error occurred', { code: 500 })
      const logs = logger.getLogs()
      expect(logs[0]!.data).toEqual({ code: 500 })
    })

    it('should limit memory logs to 100 entries', () => {
      for (let i = 0; i < 110; i++) {
        logger.info(`message ${i}`)
      }
      const logs = logger.getLogs()
      expect(logs).toHaveLength(100)
      expect(logs[0]!.message).toBe('message 10')
      expect(logs[99]!.message).toBe('message 109')
    })

    it('should support all log levels', () => {
      logger.debug('debug msg')
      logger.info('info msg')
      logger.warn('warn msg')
      logger.error('error msg')

      const logs = logger.getLogs()
      expect(logs).toHaveLength(4)
      expect(logs[0]!.level).toBe('debug')
      expect(logs[1]!.level).toBe('info')
      expect(logs[2]!.level).toBe('warn')
      expect(logs[3]!.level).toBe('error')
    })
  })

  describe('clear', () => {
    it('should clear all memory logs', () => {
      logger.info('test1')
      logger.info('test2')
      expect(logger.getLogs()).toHaveLength(2)

      logger.clear()
      expect(logger.getLogs()).toHaveLength(0)
    })
  })

  describe('getLogs', () => {
    it('should return a copy of logs', () => {
      logger.info('test')
      const logs1 = logger.getLogs()
      const logs2 = logger.getLogs()
      expect(logs1).not.toBe(logs2) // 不同引用
      expect(logs1).toEqual(logs2) // 相同内容
    })
  })
})
