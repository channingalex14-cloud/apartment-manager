/**
 * 缴费类型定义
 */

export interface Payment {
  id: number;
  bill_id: number | null;
  room_id: number;
  lease_id: number | null;
  amount: number;
  payment_date: string | null;
  payment_method: string | null;
  wechat_amount: number;
  alipay_amount: number;
  cash_amount: number;
  bank_amount: number;
  deposit_deduct_amount: number;
  payer_name: string | null;
  confirmation_screenshot: string | null;
  operator: string | null;
  notes: string | null;
  is_deleted: boolean;
  created_at: string | null;
}

export interface PaymentResponse {
  success: boolean;
  payment_id: number | null;
  message: string | null;
}

export type PaymentMethod =
  | "微信"
  | "支付宝"
  | "银行卡"
  | "现金"
  | "商家码"
  | "押金抵扣"
  | "混合支付";
