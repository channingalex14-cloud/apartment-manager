/**
 * 配置 API 服务
 * Phase 4: 权限系统基础 — set() 需要 admin token
 */

import { callCommand } from "./api";
import { authService } from "./auth.service";
import type { SystemConfig } from "@/types/config";

export const configService = {
  /** 获取所有配置（无需权限） */
  async getAll(): Promise<SystemConfig[]> {
    return callCommand<SystemConfig[]>("get_config");
  },

  /** 设置配置（需要 admin 权限） */
  async set(key: string, value: string): Promise<boolean> {
    const token = authService.getToken();
    if (!token) {
      throw new Error("未登录或会话已过期");
    }

    const result = await callCommand<{ success: boolean; message?: string }>("set_config", {
      token,
      key,
      value,
    });

    if (!result.success && result.message) {
      throw new Error(result.message);
    }
    return result.success;
  },
};
