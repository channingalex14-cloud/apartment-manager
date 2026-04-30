/**
 * 备份服务
 */

import { callCommand } from "./api";

export interface BackupSettings {
  auto_backup_enabled: boolean;
  retention_count: number;
  backup_dir: string;
}

export interface BackupInfo {
  filename: string;
  path: string;
  size_bytes: number;
  created_at: string;
}

export interface BackupResponse {
  success: boolean;
  backup_path: string;
  message: string;
}

export const backupService = {
  /** 手动执行数据库备份 */
  backup: (): Promise<BackupResponse> => {
    return callCommand<BackupResponse>("backup_database");
  },

  /** 列出所有备份文件 */
  listBackups: (): Promise<BackupInfo[]> => {
    return callCommand<BackupInfo[]>("list_backups");
  },

  /** 从备份文件恢复数据库 */
  restoreBackup: (backupPath: string): Promise<string> => {
    return callCommand<string>("restore_backup", { backup_path: backupPath });
  },

  /** 删除单个备份文件 */
  deleteBackup: (backupPath: string): Promise<void> => {
    return callCommand<void>("delete_backup", { backup_path: backupPath });
  },

  /** 获取备份设置 */
  getSettings: (): Promise<BackupSettings> => {
    return callCommand<BackupSettings>("get_backup_settings");
  },

  /** 保存备份设置 */
  saveSettings: (settings: BackupSettings): Promise<void> => {
    return callCommand<void>("save_backup_settings", { settings });
  },
};
