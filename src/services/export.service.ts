/**
 * 导出服务
 */

import { callCommand } from "./api";

export interface ExportData {
  exportType: string;
  exportTime: string;
  recordCount: number;
  data: unknown;
}

export type ExportType = "rooms" | "tenants" | "bills" | "payments" | "summary";

export const exportService = {
  /**
   * 导出数据为 JSON 字符串
   * @param type 导出类型：rooms | tenants | bills | payments | summary
   * @param yearMonth 可选的年月筛选（用于 bills 和 payments）
   */
  exportData: (type: ExportType, yearMonth?: string): Promise<string> => {
    return callCommand<string>("export_data", { data_type: type, year_month: yearMonth });
  },
};
