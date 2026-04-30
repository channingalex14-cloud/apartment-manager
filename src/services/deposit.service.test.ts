import { describe, it, expect, vi, beforeEach } from 'vitest'
import { depositService } from './deposit.service'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock retry utility
vi.mock('@/utils/retry', () => ({
  retryPromise: vi.fn((fn) => fn()),
}))

import { invoke } from '@tauri-apps/api/core'

const mockInvoke = vi.mocked(invoke)

describe('depositService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('getLedger', () => {
    it('should call get_deposit_ledger without filters', async () => {
      const mockResponse = {
        entries: [
          {
            lease_id: 1,
            room_number: '101',
            tenant_name: '张三',
            deposit_amount: 400000,
            deductions: 0,
            balance: 400000,
          },
        ],
        total: 1,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await depositService.getLedger()

      expect(mockInvoke).toHaveBeenCalledWith('get_deposit_ledger', {
        leaseId: null,
        roomId: null,
      })
      expect(result).toEqual(mockResponse)
    })

    it('should call get_deposit_ledger with leaseId filter', async () => {
      const mockResponse = {
        entries: [
          {
            lease_id: 1,
            room_number: '101',
            tenant_name: '张三',
            deposit_amount: 400000,
            deductions: 50000,
            balance: 350000,
          },
        ],
        total: 1,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await depositService.getLedger(1)

      expect(mockInvoke).toHaveBeenCalledWith('get_deposit_ledger', {
        leaseId: 1,
        roomId: null,
      })
      expect(result).toEqual(mockResponse)
    })

    it('should call get_deposit_ledger with roomId filter', async () => {
      const mockResponse = {
        entries: [
          {
            lease_id: 1,
            room_number: '101',
            tenant_name: '张三',
            deposit_amount: 400000,
            deductions: 0,
            balance: 400000,
          },
        ],
        total: 1,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await depositService.getLedger(undefined, 101)

      expect(mockInvoke).toHaveBeenCalledWith('get_deposit_ledger', {
        leaseId: null,
        roomId: 101,
      })
      expect(result).toEqual(mockResponse)
    })

    it('should call get_deposit_ledger with both filters', async () => {
      const mockResponse = {
        entries: [
          {
            lease_id: 1,
            room_number: '101',
            tenant_name: '张三',
            deposit_amount: 400000,
            deductions: 0,
            balance: 400000,
          },
        ],
        total: 1,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await depositService.getLedger(1, 101)

      expect(mockInvoke).toHaveBeenCalledWith('get_deposit_ledger', {
        leaseId: 1,
        roomId: 101,
      })
      expect(result).toEqual(mockResponse)
    })

    it('should return empty entries for non-matching filters', async () => {
      const mockResponse = {
        records: [],
        total_balance: 0,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await depositService.getLedger(999)

      expect(result.records).toHaveLength(0)
      expect(result.total_balance).toBe(0)
    })
  })
})
