import { describe, it, expect } from 'vitest'
import {
  ClaudeError,
  AbortError,
  NetworkError,
  ValidationError,
  isAbortError,
  isNetworkError,
  isENOENT,
  isFsInaccessible,
  isValidationError,
  toError,
  errorMessage,
  getHttpStatus,
  isNotFoundError,
  classifyTauriError,
} from './errors'

describe('Error classes', () => {
  it('should create ClaudeError', () => {
    const err = new ClaudeError('test error')
    expect(err).toBeInstanceOf(Error)
    expect(err.name).toBe('ClaudeError')
    expect(err.message).toBe('test error')
  })

  it('should create AbortError with default message', () => {
    const err = new AbortError()
    expect(err.name).toBe('AbortError')
    expect(err.message).toBe('操作已取消')
  })

  it('should create NetworkError with status', () => {
    const err = new NetworkError('network failed', 500)
    expect(err.name).toBe('NetworkError')
    expect(err.status).toBe(500)
  })

  it('should create ValidationError with field', () => {
    const err = new ValidationError('invalid', 'email')
    expect(err.name).toBe('ValidationError')
    expect(err.field).toBe('email')
  })
})

describe('isAbortError', () => {
  it('should detect AbortError instance', () => {
    expect(isAbortError(new AbortError())).toBe(true)
  })

  it('should detect Error with AbortError name', () => {
    const err = new Error()
    err.name = 'AbortError'
    expect(isAbortError(err)).toBe(true)
  })

  it('should detect CanceledError', () => {
    expect(isAbortError({ name: 'CanceledError' })).toBe(true)
  })

  it('should return false for other errors', () => {
    expect(isAbortError(new Error('test'))).toBe(false)
    expect(isAbortError(null)).toBe(false)
  })
})

describe('isNetworkError', () => {
  it('should detect NetworkError instance', () => {
    expect(isNetworkError(new NetworkError('test'))).toBe(true)
  })

  it('should detect Error with NetworkError name', () => {
    const err = new Error()
    err.name = 'NetworkError'
    expect(isNetworkError(err)).toBe(true)
  })
})

describe('isENOENT', () => {
  it('should detect ENOENT code', () => {
    const err = new Error('not found') as Error & { code: string }
    err.code = 'ENOENT'
    expect(isENOENT(err)).toBe(true)
  })

  it('should detect Windows errno -4058', () => {
    const err = new Error('not found') as Error & { errno: number }
    err.errno = -4058
    expect(isENOENT(err)).toBe(true)
  })

  it('should return false for other errors', () => {
    expect(isENOENT(new Error('test'))).toBe(false)
  })
})

describe('isFsInaccessible', () => {
  it('should detect EACCES', () => {
    const err = new Error('access denied') as Error & { code: string }
    err.code = 'EACCES'
    expect(isFsInaccessible(err)).toBe(true)
  })

  it('should detect EPERM', () => {
    const err = new Error('permission denied') as Error & { code: string }
    err.code = 'EPERM'
    expect(isFsInaccessible(err)).toBe(true)
  })

  it('should detect Windows errno -13', () => {
    const err = new Error('denied') as Error & { errno: number }
    err.errno = -13
    expect(isFsInaccessible(err)).toBe(true)
  })
})

describe('toError', () => {
  it('should return Error instance as-is', () => {
    const err = new Error('test')
    expect(toError(err)).toBe(err)
  })

  it('should wrap string in Error', () => {
    const err = toError('string error')
    expect(err).toBeInstanceOf(Error)
    expect(err.message).toBe('string error')
  })

  it('should stringify other types', () => {
    const err = toError({ foo: 'bar' })
    expect(err.message).toBe('[object Object]')
  })
})

describe('errorMessage', () => {
  it('should return cancel message for AbortError', () => {
    expect(errorMessage(new AbortError())).toBe('操作已取消')
  })

  it('should extract message from Error', () => {
    expect(errorMessage(new Error('test message'))).toBe('test message')
  })

  it('should return string as-is', () => {
    expect(errorMessage('string error')).toBe('string error')
  })

  it('should return default for unknown', () => {
    expect(errorMessage(null)).toBe('发生了未知错误')
  })
})

describe('getHttpStatus', () => {
  it('should extract status from response', () => {
    expect(getHttpStatus({ response: { status: 404 } })).toBe(404)
  })

  it('should extract status directly', () => {
    expect(getHttpStatus({ status: 500 })).toBe(500)
  })

  it('should return undefined for non-objects', () => {
    expect(getHttpStatus(null)).toBeUndefined()
    expect(getHttpStatus('string')).toBeUndefined()
  })
})

describe('isNotFoundError', () => {
  it('should detect 404 status', () => {
    expect(isNotFoundError({ status: 404 })).toBe(true)
  })

  it('should detect "不存在" in message', () => {
    expect(isNotFoundError(new Error('账单不存在'))).toBe(true)
  })
})

describe('classifyTauriError', () => {
  it('should classify ENOENT as not_found', () => {
    const err = new Error() as Error & { code: string }
    err.code = 'ENOENT'
    expect(classifyTauriError(err)).toEqual({
      kind: 'not_found',
      message: '文件或数据不存在',
    })
  })

  it('should classify 401 as permission', () => {
    expect(classifyTauriError({ status: 401 })).toEqual({
      kind: 'permission',
      message: '无权限执行此操作',
    })
  })

  it('should classify 500 as unknown', () => {
    expect(classifyTauriError({ status: 500 })).toEqual({
      kind: 'unknown',
      message: '服务器内部错误，请稍后重试',
    })
  })

  it('should classify unknown errors', () => {
    expect(classifyTauriError(new Error('test'))).toEqual({
      kind: 'unknown',
      message: 'test',
    })
  })
})
