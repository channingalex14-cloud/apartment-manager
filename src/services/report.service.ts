/**
 * 报表 API 服务
 */

import { callCommand } from "./api";

export interface MonthlySummaryCache {
  id: number;
  yearMonth: string;
  totalRooms: number;
  rentedCount: number;
  newRentedCount: number;
  vacantCount: number;
  violationCount: number;
  staffCount: number;
  managementCount: number;
  rentTotal: number;
  propertyTotal: number;
  waterTotal: number;
  electricTotal: number;
  managementTotal: number;
  repairTotal: number;
  depositTotal: number;
  previousBalanceTotal: number;
  actualPaidTotal: number;
  occupancyRate: number;
  generatedAt: string | null;
  updatedAt: string | null;
}

export interface MonthlySummaryResponse {
  success: boolean;
  data: MonthlySummaryCache | null;
  message: string | null;
}

export interface MonthlySummaryListResponse {
  success: boolean;
  data: MonthlySummaryCache[];
  message: string | null;
}

export const reportService = {
  /** 生成月度汇总缓存 */
  async generateMonthlySummary(yearMonth: string): Promise<MonthlySummaryResponse> {
    return callCommand<MonthlySummaryResponse>("generate_monthly_summary", { year_month: yearMonth });
  },

  /** 获取月度汇总 */
  async getSummaryReport(yearMonth: string): Promise<MonthlySummaryResponse> {
    return callCommand<MonthlySummaryResponse>("get_summary_report", { year_month: yearMonth });
  },

  /** 列出所有月度汇总 */
  async listSummaryReports(): Promise<MonthlySummaryListResponse> {
    return callCommand<MonthlySummaryListResponse>("list_summary_reports");
  },
};
