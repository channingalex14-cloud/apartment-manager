/**
 * 账单类型定义
 */

export interface MonthlyBill {
  id: number;
  year_month: string;
  room_id: number;
  lease_id: number | null;
  lease_start_date: string | null;
  lease_end_date: string | null;
  check_in_day: number | null;
  check_out_day: number | null;
  water_reading_prev: number;
  water_reading_current: number;
  electric_reading_prev: number;
  electric_reading_current: number;
  water_usage: number;
  electric_usage: number;
  water_unit_price: number;
  electric_unit_price: number;
  management_unit_price: number;
  rent_fee: number;
  rent_days: number;
  rent_daily_rate: number;
  property_fee: number;
  water_fee: number;
  electric_fee: number;
  management_fee: number;
  repair_fee: number;
  misc_fee: number;
  misc_fee_remark: string | null;
  deposit_fee: number;
  previous_balance: number;
  actual_paid: number;
  total_amount: number;
  bill_type: string;
  room_status: string;
  status: string;
  due_date: string | null;
  paid_date: string | null;
  bill_sequence: number;
  is_deleted: boolean;
  is_archived: boolean;  // 是否已归档
  archived_at: string | null;  // 归档时间
  notes: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface BillResponse {
  success: boolean;
  generated_count: number;
  message: string | null;
}

export type BillStatus = "待缴费" | "已支付" | "部分支付" | "已作废";

export type BillType =
  | "正常"
  | "首月免水电"
  | "半月结算"
  | "末月结算"
  | "月中退房结算";

/** 账单列表项 */
export interface BillListItem {
  id: number;
  room_id: number;
  room_number: string;
  building: string;
  tenant_name: string | null;
  year_month: string;
  total_amount: number;
  actual_paid: number;
  status: string;
  due_date: string | null;
}

/** 账单列表响应 */
export interface BillListResponse {
  bills: BillListItem[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

/** 账单详情响应 */
export interface BillDetailResponse {
  id: number;
  room_id: number;
  room_number: string;
  building: string;
  tenant_name: string | null;
  tenant_phone: string | null;
  year_month: string;
  status: string;
  due_date: string | null;
  created_at: string | null;
  // 费用明细（分）
  rent_fee: number;
  property_fee: number;
  water_fee: number;
  electric_fee: number;
  management_fee: number;
  misc_fee: number;
  misc_fee_remark: string | null;
  previous_balance: number;
  total_amount: number;
  actual_paid: number;
  remaining_amount: number;
  // 水电读数
  water_reading_prev: number;
  water_reading_current: number;
  electric_reading_prev: number;
  electric_reading_current: number;
  repair_fee: number;
  // 操作权限
  can_confirm: boolean;
  can_void: boolean;
  can_pay_partial: boolean;
}

/** 账单操作响应 */
export interface BillActionResponse {
  success: boolean;
  message: string;
}
