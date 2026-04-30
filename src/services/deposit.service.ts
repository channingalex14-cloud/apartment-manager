/**
 * 押金 API 服务
 */

import { callCommand } from "./api";
import type { DepositLedgerResponse } from "@/types/deposit";

export const depositService = {
  async getLedger(leaseId?: number, roomId?: number): Promise<DepositLedgerResponse> {
    return callCommand<DepositLedgerResponse>("get_deposit_ledger", {
      lease_id: leaseId || null,
      room_id: roomId || null,
    });
  },

  async receiveDeposit(leaseId: number, amount: number, operator?: string, notes?: string): Promise<string> {
    return callCommand<string>("receive_deposit", { lease_id: leaseId, amount, operator, notes });
  },

  async refundDeposit(leaseId: number, amount: number, operator?: string, notes?: string): Promise<string> {
    return callCommand<string>("refund_deposit", { lease_id: leaseId, amount, operator, notes });
  },
};
