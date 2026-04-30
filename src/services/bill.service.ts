/**
 * 账单 API 服务
 */

import { callCommand } from "./api";
import type {
  MonthlyBill,
  BillResponse,
  BillListResponse,
  BillDetailResponse,
  BillActionResponse,
} from "@/types/bill";

export interface GenerateBillsRequest {
  year_month: string;
  room_ids?: number[];
  operator?: string;
  misc_fee?: number;
  misc_fee_remark?: string;
}

export const billService = {
  /** 列出账单 */
  async list(yearMonth?: string): Promise<MonthlyBill[]> {
    return callCommand<MonthlyBill[]>("list_bills", { year_month: yearMonth });
  },

  /** 生成月度账单 */
  async generate(req: GenerateBillsRequest): Promise<BillResponse> {
    return callCommand<BillResponse>("generate_monthly_bills", { req });
  },

  /** 查询账单列表（分页 + 筛选） */
  async queryBills(params: {
    year?: number;
    month?: number;
    roomId?: number;
    status?: string;
    page: number;
    pageSize: number;
  }): Promise<BillListResponse> {
    return callCommand<BillListResponse>("query_bills", {
      year: params.year ?? undefined,
      month: params.month ?? undefined,
      room_id: params.roomId ?? undefined,
      status: params.status ?? undefined,
      page: params.page,
      page_size: params.pageSize,
    });
  },

  /** 查询账单详情 */
  async getBillDetail(billId: number): Promise<BillDetailResponse> {
    return callCommand<BillDetailResponse>("get_bill_detail", { bill_id: billId });
  },

  /** 确认全额支付 */
  async confirmBillPaid(billId: number): Promise<BillActionResponse> {
    return callCommand<BillActionResponse>("confirm_bill_paid", { bill_id: billId });
  },

  /** 部分支付 */
  async partialPayBill(billId: number, amount: number): Promise<BillActionResponse> {
    return callCommand<BillActionResponse>("partial_pay_bill", { bill_id: billId, amount });
  },

  /** 作废账单 */
  async voidBill(billId: number): Promise<BillActionResponse> {
    return callCommand<BillActionResponse>("void_bill", { bill_id: billId });
  },

  /** 归档指定年月的账单 */
  async archiveBills(yearMonth: string): Promise<{ success: boolean; archived_count: number; message?: string }> {
    return callCommand("archive_bills", { year_month: yearMonth });
  },

  /** 恢复指定年月的已归档账单 */
  async restoreBills(yearMonth: string): Promise<{ success: boolean; archived_count: number; message?: string }> {
    return callCommand("restore_bills", { year_month: yearMonth });
  },

  /** 获取所有已归档的年月列表 */
  async listArchivedMonths(): Promise<string[]> {
    return callCommand<string[]>("list_archived_months");
  },
};
