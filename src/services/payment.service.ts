/**
 * 缴费 API 服务
 */

import { callCommand } from "./api";
import type { Payment, PaymentResponse } from "@/types/payment";

export interface RecordPaymentRequest {
  bill_id: number;
  amount: number;
  payment_method: string;
  payment_date: string;
  payer_name?: string;
  wechat_amount?: number;
  alipay_amount?: number;
  cash_amount?: number;
  bank_amount?: number;
  deposit_deduct_amount?: number;
  operator?: string;
}

export const paymentService = {
  /** 列出缴费记录 */
  async list(billId?: number): Promise<Payment[]> {
    return callCommand<Payment[]>("list_payments", { bill_id: billId });
  },

  /** 记录缴费 */
  async record(req: RecordPaymentRequest): Promise<PaymentResponse> {
    return callCommand<PaymentResponse>("record_payment", { req });
  },

  /** 作废缴费记录（红冲） */
  async void(paymentId: number, operator?: string): Promise<PaymentResponse> {
    return callCommand<PaymentResponse>("void_payment", { payment_id: paymentId, operator });
  },

  /** 更新付款方式 */
  async updateMethod(paymentId: number, paymentMethod: string, operator?: string): Promise<PaymentResponse> {
    return callCommand<PaymentResponse>("update_payment_method", { payment_id: paymentId, payment_method: paymentMethod, operator });
  },
};
