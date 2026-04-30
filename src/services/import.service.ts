/**
 * 导入 API 服务
 */

import { callCommand } from "./api";

export interface ImportBillRequest {
  file_path: string;
  year_month: string;
  operator?: string;
}

export interface ImportBillResponse {
  success: boolean;
  imported_count: number;
  skipped_count: number;
  errors: string[];
  message: string | null;
}

export const importService = {
  /**
   * 从 Excel 账单文件导入月度数据
   *
   * @param filePath Excel 文件的完整路径
   * @param yearMonth 年月，如 "2026-03"
   * @param operator 操作员（可选）
   */
  async importMonthlyBills(
    filePath: string,
    yearMonth: string,
    operator?: string
  ): Promise<ImportBillResponse> {
    return callCommand<ImportBillResponse>("import_monthly_bills", {
      req: {
        file_path: filePath,
        year_month: yearMonth,
        operator,
      },
    });
  },
};
