import { describe, it, expect, vi, beforeEach } from 'vitest'
import { leaseService } from './lease.service'

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

describe('leaseService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('list', () => {
    it('should call list_leases command', async () => {
      const mockLeases = [
        { id: 1, room_id: 1, tenant_id: 1, status: 'active' },
        { id: 2, room_id: 2, tenant_id: 2, status: 'pending' },
      ]
      mockInvoke.mockResolvedValueOnce(mockLeases)

      const result = await leaseService.list()

      expect(mockInvoke).toHaveBeenCalledWith('list_leases', undefined)
      expect(result).toEqual(mockLeases)
    })
  })

  describe('get', () => {
    it('should call get_lease command', async () => {
      const mockLease = { id: 1, room_id: 1, tenant_id: 1 }
      mockInvoke.mockResolvedValueOnce(mockLease)

      const result = await leaseService.get(1)

      expect(mockInvoke).toHaveBeenCalledWith('get_lease', { id: 1 })
      expect(result).toEqual(mockLease)
    })

    it('should return null for non-existent lease', async () => {
      mockInvoke.mockResolvedValueOnce(null)

      const result = await leaseService.get(999)

      expect(result).toBeNull()
    })
  })

  describe('create', () => {
    it('should call create_lease command with correct params', async () => {
      mockInvoke.mockResolvedValueOnce(1) // 返回新创建的 lease_id

      const result = await leaseService.create({
        room_id: 1,
        tenant_id: 2,
        start_date: '2026-04-01',
        monthly_rent: 200000, // 分为单位
        property_fee: 10000,
        deposit: 400000,
        contract_number: 'XY-2026-001',
        end_date: '2027-03-31',
      })

      expect(mockInvoke).toHaveBeenCalledWith('create_lease', {
        roomId: 1,
        tenantId: 2,
        startDate: '2026-04-01',
        monthlyRent: 200000,
        propertyFee: 10000,
        deposit: 400000,
        contractNumber: 'XY-2026-001',
        endDate: '2027-03-31',
      })
      expect(result).toBe(1)
    })
  })

  describe('activate', () => {
    it('should call activate_lease command', async () => {
      mockInvoke.mockResolvedValueOnce(true)

      const result = await leaseService.activate(1)

      expect(mockInvoke).toHaveBeenCalledWith('activate_lease', { id: 1 })
      expect(result).toBe(true)
    })
  })

  describe('checkIn', () => {
    it('should call check_in command', async () => {
      const mockResponse = { success: true, message: '入住成功' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await leaseService.checkIn({
        room_id: 1,
        tenant_id: 2,
        lease_id: 3,
        move_in_date: '2026-04-01',
        operator: 'admin',
      })

      expect(mockInvoke).toHaveBeenCalledWith('check_in', {
        req: {
          room_id: 1,
          tenant_id: 2,
          lease_id: 3,
          move_in_date: '2026-04-01',
          operator: 'admin',
        },
      })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('checkOut', () => {
    it('should call check_out command', async () => {
      const mockResponse = { success: true, message: '退房成功' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await leaseService.checkOut({
        lease_id: 1,
        room_id: 2,
        move_out_date: '2026-04-30',
        reason: '合同到期',
        operator: 'admin',
      })

      expect(mockInvoke).toHaveBeenCalledWith('check_out', {
        req: {
          lease_id: 1,
          room_id: 2,
          move_out_date: '2026-04-30',
          reason: '合同到期',
          operator: 'admin',
        },
      })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('markViolation', () => {
    it('should call mark_violation command', async () => {
      const mockResponse = { success: true, message: '已标记违约' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await leaseService.markViolation(1)

      expect(mockInvoke).toHaveBeenCalledWith('mark_violation', { leaseId: 1 })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('recoverFromViolation', () => {
    it('should call recover_from_violation command', async () => {
      const mockResponse = { success: true, message: '已恢复正常' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await leaseService.recoverFromViolation(1)

      expect(mockInvoke).toHaveBeenCalledWith('recover_from_violation', { leaseId: 1 })
      expect(result).toEqual(mockResponse)
    })
  })
})
