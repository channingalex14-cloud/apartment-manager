/**
 * 租客 API 服务
 */

import { callCommand } from "./api";
import type { TenantResponse } from "@/types/tenant";

export interface CreateTenantRequest {
  name: string;
  phone: string;
  phone2?: string;
  emergency_contact?: string;
  emergency_phone?: string;
}

export interface UpdateTenantRequest {
  name?: string;
  phone?: string;
  phone2?: string;
  emergency_contact?: string;
  emergency_phone?: string;
}

export interface TenantHistoryItem {
  id: number;
  tenant_id: number;
  event_type: string;
  room_id: number | null;
  lease_id: number | null;
  event_date: string;
  old_value: string | null;
  new_value: string | null;
  notes: string | null;
  created_at: string | null;
}

export const tenantService = {
  /** 列出所有租客 */
  async list(): Promise<TenantResponse[]> {
    return callCommand<TenantResponse[]>("list_tenants");
  },

  /** 获取租客详情 */
  async get(id: number): Promise<TenantResponse | null> {
    return callCommand<TenantResponse | null>("get_tenant", { id });
  },

  /** 创建租客 */
  async create(req: CreateTenantRequest): Promise<number> {
    return callCommand<number>("create_tenant", { req });
  },

  /** 更新租客 */
  async update(id: number, req: UpdateTenantRequest): Promise<boolean> {
    return callCommand<boolean>("update_tenant", {
      id,
      name: req.name,
      phone: req.phone,
      phone2: req.phone2,
      emergency_contact: req.emergency_contact,
      emergency_phone: req.emergency_phone,
    });
  },

  /** 获取租客历史 */
  async getHistory(tenantId: number): Promise<TenantHistoryItem[]> {
    return callCommand<TenantHistoryItem[]>("get_tenant_history", { tenant_id: tenantId });
  },

  async delete(id: number): Promise<boolean> {
    return callCommand<boolean>("delete_tenant", { id });
  },
};
