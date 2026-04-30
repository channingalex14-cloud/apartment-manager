/**
 * 押金类型定义
 */

export interface DepositLedger {
  id: number;
  lease_id: number;
  room_id: number;
  transaction_type: string;
  amount: number;
  balance: number;
  reference_bill_id: number | null;
  reference_payment_id: number | null;
  operator: string | null;
  transaction_date: string | null;
  notes: string | null;
  created_at: string | null;
}

export interface DepositLedgerItem {
  id: number;
  room_number: string;
  tenant_name: string | null;
  transaction_type: string;
  amount_fen: number;
  balance_fen: number;
  transaction_date: string | null;
  operator: string | null;
  notes: string | null;
}

export interface DepositLedgerResponse {
  records: DepositLedgerItem[];
  total_balance: number;
}
