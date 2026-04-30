/**
 * 租客类型定义
 */

export interface Tenant {
  id: number;
  name: string;
  phone: string;
  phone2: string | null;
  emergency_contact: string | null;
  emergency_phone: string | null;
  is_deleted: boolean;
  created_at: string | null;
  updated_at: string | null;
}

export interface TenantResponse {
  id: number;
  name: string;
  phone: string;
  phone2: string | null;
  emergency_contact: string | null;
  emergency_phone: string | null;
}
