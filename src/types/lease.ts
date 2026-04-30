/**
 * 合同类型定义
 */

export interface Lease {
  id: number;
  room_id: number;
  tenant_id: number;
  contract_number: string | null;
  start_date: string;
  end_date: string | null;
  monthly_rent: number;
  property_fee: number;
  deposit: number;
  deposit_received: number;
  deposit_balance: number;
  deposit_status: string;
  move_in_date: string | null;
  move_out_date: string | null;
  termination_reason: string | null;
  status: string;
  status_reason: string | null;
  notes: string | null;
  is_deleted: boolean;
  created_at: string | null;
  updated_at: string | null;
}

export interface LeaseResponse {
  success: boolean;
  lease_id: number | null;
  message: string | null;
}

export type LeaseStatus =
  | "草稿"
  | "生效中"
  | "违约中"
  | "待结算"
  | "已退房"
  | "已作废";

export type DepositStatus =
  | "未收取"
  | "已收取"
  | "部分收取"
  | "退还"
  | "没收";
