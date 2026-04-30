/**
 * 基础 API 服务
 *
 * 所有 API 调用都通过 invoke() 发送到 Rust 后端
 * 前端禁止直接操作数据库
 */

import { invoke } from "@tauri-apps/api/core";
import { retryPromise } from "@/utils/retry";
import { authService } from "@/services/auth.service";

export async function callCommand<T>(
  command: string,
  args?: Record<string, unknown>,
  retries = 3,
): Promise<T> {
  const token = authService.getToken();
  const finalArgs = token ? { ...args, token } : args;
  return retryPromise(
    () => invoke<T>(command, finalArgs),
    retries,
    500,
  )
}

