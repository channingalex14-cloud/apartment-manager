/**
 * 维护 API 服务
 */

import { callCommand } from "./api";

export const maintenanceService = {
  /** 执行数据库 VACUUM（压缩） */
  async vacuumDatabase(): Promise<string> {
    return callCommand<string>("vacuum_database_cmd");
  },
};
