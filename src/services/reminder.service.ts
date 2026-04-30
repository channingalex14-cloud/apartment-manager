/**
 * 提醒 API 服务
 */

import { callCommand } from "./api";

export interface Reminder {
  id: number;
  reminder_type: string;
  room_id: number | null;
  lease_id: number | null;
  title: string;
  message: string | null;
  scheduled_date: string | null;
  reminded_at: string | null;
  is_sent: boolean;
  is_read: boolean;
  created_at: string | null;
}

export interface CreateReminderRequest {
  reminder_type: string;
  room_id?: number;
  lease_id?: number;
  title: string;
  message?: string;
  scheduled_date?: string;
}

export interface UpdateReminderStatusRequest {
  is_sent?: boolean;
  is_read?: boolean;
}

export interface ReminderResponse {
  success: boolean;
  reminder_id: number | null;
  message: string | null;
}

export interface ReminderListResponse {
  success: boolean;
  data: Reminder[];
  message: string | null;
}

export const reminderService = {
  /** 创建提醒 */
  async create(req: CreateReminderRequest): Promise<ReminderResponse> {
    return callCommand<ReminderResponse>("create_reminder", { req });
  },

  /** 获取提醒列表 */
  async list(roomId?: number, isRead?: boolean): Promise<ReminderListResponse> {
    return callCommand<ReminderListResponse>("list_reminders", { room_id: roomId, is_read: isRead });
  },

  /** 获取待发送的提醒 */
  async getPending(): Promise<ReminderListResponse> {
    return callCommand<ReminderListResponse>("get_pending_reminders");
  },

  /** 更新提醒状态 */
  async updateStatus(id: number, req: UpdateReminderStatusRequest): Promise<ReminderResponse> {
    return callCommand<ReminderResponse>("update_reminder_status", { id, req });
  },

  /** 标记提醒为已发送 */
  async markSent(id: number): Promise<ReminderResponse> {
    return callCommand<ReminderResponse>("mark_reminder_sent", { id });
  },

  /** 标记提醒为已读 */
  async markRead(id: number): Promise<ReminderResponse> {
    return callCommand<ReminderResponse>("mark_reminder_read", { id });
  },

  /** 删除提醒 */
  async delete(id: number): Promise<ReminderResponse> {
    return callCommand<ReminderResponse>("delete_reminder", { id });
  },
};
