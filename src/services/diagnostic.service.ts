/**
 * 诊断 API 服务
 */

import { callCommand } from "./api";

export interface ExcelDiagnostic {
  sheet_names: string[];
  total_rows: number;
  header_row: string[];
  first_data_row: string[] | null;
  room_numbers_sample: string[];
  all_room_numbers: string[];
}

export interface DuplicateRoom {
  room_number: string;
  count: number;
  ids: number[];
}

export interface DbDiagnostic {
  duplicate_rooms: DuplicateRoom[];
  room_type_distribution: [string, number][];
  total_rooms: number;
}

export const diagnosticService = {
  /** 诊断 Excel 文件 */
  async diagnoseExcel(filePath: string): Promise<ExcelDiagnostic> {
    return callCommand<ExcelDiagnostic>("diagnose_excel_file", { file_path: filePath });
  },

  /** 诊断数据库（JSON） */
  async diagnoseDatabase(): Promise<DbDiagnostic> {
    return callCommand<DbDiagnostic>("diagnose_database", {});
  },

  /** 诊断数据库（文本） */
  async diagnoseDatabaseText(): Promise<string> {
    return callCommand<string>("diagnose_database_text", {});
  },

  /** 查询指定房间的详细信息（rooms + leases + tenants） */
  async diagnoseRoomDetail(roomNumber: string): Promise<string> {
    return callCommand<string>("diagnose_room_detail", { room_number: roomNumber });
  },

  /** 修复管理/员工房间的租客关联（通用版本）
   *  @param fixes - 房间-租客映射列表 [(房间号, 租客名, 电话), ...]
   */
  async fixManagementRooms(fixes: [string, string, string][]): Promise<string> {
    return callCommand<string>("fix_management_rooms", { fixes });
  },

  /** 查询房间301的详细信息（调试用） */
  async queryRoom301(): Promise<string> {
    return callCommand<string>("query_room_301", {});
  },
};
