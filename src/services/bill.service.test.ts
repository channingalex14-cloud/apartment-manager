import { describe, it, expect, vi, beforeEach } from 'vitest'
import { billService } from './bill.service'

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

describe('billService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('list', () => {
    it('should call list_bills command', async () => {
      const mockBills = [
        { id: 1, year_month: '2026-04', status: '待缴费' },
        { id: 2, year_month: '2026-03', status: '已支付' },
      ]
      mockInvoke.mockResolvedValueOnce(mockBills)

      const result = await billService.list('2026-04')

      expect(mockInvoke).toHaveBeenCalledWith('list_bills', { yearMonth: '2026-04' })
      expect(result).toEqual(mockBills)
    })

    it('should call list_bills without yearMonth', async () => {
      mockInvoke.mockResolvedValueOnce([])

      await billService.list()

      expect(mockInvoke).toHaveBeenCalledWith('list_bills', { yearMonth: undefined })
    })
  })

  describe('generate', () => {
    it('should call generate_monthly_bills command', async () => {
      const mockResponse = { success: true, generated_count: 5 }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await billService.generate({
        year_month: '2026-04',
        room_ids: [1, 2, 3],
        operator: 'admin',
      })

      expect(mockInvoke).toHaveBeenCalledWith('generate_monthly_bills', {
        req: {
          year_month: '2026-04',
          room_ids: [1, 2, 3],
          operator: 'admin',
        },
      })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('queryBills', () => {
    it('should call query_bills with pagination', async () => {
      const mockResponse = {
        bills: [{ id: 1, total_amount: 1000 }],
        total: 1,
        page: 1,
        page_size: 10,
      }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await billService.queryBills({
        page: 1,
        pageSize: 10,
      })

      expect(mockInvoke).toHaveBeenCalledWith('query_bills', {
        year: undefined,
        month: undefined,
        roomId: undefined,
        status: undefined,
        page: 1,
        pageSize: 10,
      })
      expect(result).toEqual(mockResponse)
    })

    it('should call query_bills with filters', async () => {
      const mockResponse = { bills: [], total: 0, page: 1, page_size: 10 }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      await billService.queryBills({
        year: 2026,
        month: 4,
        roomId: 5,
        status: '待缴费',
        page: 1,
        pageSize: 10,
      })

      expect(mockInvoke).toHaveBeenCalledWith('query_bills', {
        year: 2026,
        month: 4,
        roomId: 5,
        status: '待缴费',
        page: 1,
        pageSize: 10,
      })
    })
  })

  describe('getBillDetail', () => {
    it('should call get_bill_detail command', async () => {
      const mockDetail = {
        bill: { id: 1, total_amount: 1000 },
        payments: [],
      }
      mockInvoke.mockResolvedValueOnce(mockDetail)

      const result = await billService.getBillDetail(1)

      expect(mockInvoke).toHaveBeenCalledWith('get_bill_detail', { billId: 1 })
      expect(result).toEqual(mockDetail)
    })
  })

  describe('confirmBillPaid', () => {
    it('should call confirm_bill_paid command', async () => {
      const mockResponse = { success: true, message: '支付成功' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await billService.confirmBillPaid(1)

      expect(mockInvoke).toHaveBeenCalledWith('confirm_bill_paid', { billId: 1 })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('partialPayBill', () => {
    it('should call partial_pay_bill command', async () => {
      const mockResponse = { success: true, message: '部分支付成功' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await billService.partialPayBill(1, 500)

      expect(mockInvoke).toHaveBeenCalledWith('partial_pay_bill', { billId: 1, amount: 500 })
      expect(result).toEqual(mockResponse)
    })
  })

  describe('voidBill', () => {
    it('should call void_bill command', async () => {
      const mockResponse = { success: true, message: '账单已作废' }
      mockInvoke.mockResolvedValueOnce(mockResponse)

      const result = await billService.voidBill(1)

      expect(mockInvoke).toHaveBeenCalledWith('void_bill', { billId: 1 })
      expect(result).toEqual(mockResponse)
    })
  })
})
