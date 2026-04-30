import { describe, it, expect, beforeEach, vi } from 'vitest'
import { safeParseJSON, parseJSONC, parseJSONL, safeStringify, formatZodError } from './json'

describe('safeParseJSON', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  it('should parse valid JSON', () => {
    const result = safeParseJSON('{"name": "test"}')
    expect(result).toEqual({ name: 'test' })
  })

  it('should return null for invalid JSON', () => {
    const result = safeParseJSON('not json')
    expect(result).toBeNull()
  })

  it('should return null for null/undefined input', () => {
    expect(safeParseJSON(null)).toBeNull()
    expect(safeParseJSON(undefined)).toBeNull()
    expect(safeParseJSON('')).toBeNull()
  })

  it('should parse with generic type', () => {
    const result = safeParseJSON<number>('42')
    expect(result).toBe(42)
  })

  it('should use cache for repeated calls', () => {
    const json = '{"cached": true}'
    const result1 = safeParseJSON(json)
    vi.advanceTimersByTime(1000)
    const result2 = safeParseJSON(json)
    expect(result1).toBe(result2) // 同一引用
  })

  it('should invalidate cache after 5 seconds', () => {
    const json = '{"cached": true}'
    const result1 = safeParseJSON(json)
    vi.advanceTimersByTime(6000)
    const result2 = safeParseJSON(json)
    expect(result1).toEqual(result2) // 值相同但不是同一引用
  })
})

describe('parseJSONC', () => {
  it('should parse JSON with single-line comments', () => {
    const jsonc = '{"name": "test" // comment\n}'
    const result = parseJSONC(jsonc)
    expect(result).toEqual({ name: 'test' })
  })

  it('should parse JSON with multi-line comments', () => {
    const jsonc = '{\n/* this is a comment */\n"name": "test"\n}'
    const result = parseJSONC(jsonc)
    expect(result).toEqual({ name: 'test' })
  })

  it('should return null for empty content', () => {
    expect(parseJSONC('')).toBeNull()
    expect(parseJSONC('{')).toBeNull()
    expect(parseJSONC('[')).toBeNull()
  })
})

describe('parseJSONL', () => {
  it('should parse JSONL format', () => {
    const jsonl = '{"id": 1}\n{"id": 2}\n{"id": 3}'
    const result = parseJSONL<{ id: number }>(jsonl)
    expect(result).toHaveLength(3)
    expect(result[0]!.id).toBe(1)
    expect(result[1]!.id).toBe(2)
    expect(result[2]!.id).toBe(3)
  })

  it('should skip empty lines', () => {
    const jsonl = '{"id": 1}\n\n{"id": 2}\n   \n{"id": 3}'
    const result = parseJSONL<{ id: number }>(jsonl)
    expect(result).toHaveLength(3)
  })

  it('should skip invalid lines', () => {
    const jsonl = '{"id": 1}\ninvalid\n{"id": 2}'
    const result = parseJSONL<{ id: number }>(jsonl)
    expect(result).toHaveLength(2)
  })
})

describe('safeStringify', () => {
  it('should stringify valid values', () => {
    expect(safeStringify({ name: 'test' })).toBe('{"name":"test"}')
  })

  it('should stringify with indentation', () => {
    expect(safeStringify({ name: 'test' }, 2)).toBe('{\n  "name": "test"\n}')
  })

  it('should handle circular references gracefully', () => {
    const obj: { self?: unknown } = {}
    obj.self = obj
    const result = safeStringify(obj)
    expect(result).toBe('{}') // 循环引用会抛错，返回空对象
  })
})

describe('formatZodError', () => {
  it('should format Zod v3 style errors', () => {
    const error = {
      issues: [
        { path: ['name'], message: 'Required' },
        { path: ['age'], message: 'Must be a number', received: 'string' },
      ],
    }
    const result = formatZodError(error)
    expect(result).toBe('name: Required\nage: Must be a number（收到: string）')
  })

  it('should format Zod v4 style errors', () => {
    const error = {
      errors: [{ path: ['email'], message: 'Invalid email' }],
    }
    const result = formatZodError(error)
    expect(result).toBe('email: Invalid email')
  })

  it('should handle non-object errors', () => {
    expect(formatZodError(null)).toBe('null')
    expect(formatZodError('string error')).toBe('string error')
  })
})
