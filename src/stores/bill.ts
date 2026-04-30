import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { callCommand } from "@/services/api";
import type { MonthlyBill, BillResponse } from "@/types/bill";

export interface GenerateBillsRequest {
  year_month: string;
  room_ids?: number[];
  operator?: string;
}

export interface BillSummary {
  total_amount: number;
  total_paid: number;
  total_pending: number;
  bill_count: number;
  pending_count: number;
  paid_count: number;
}

export const useBillStore = defineStore("bill", () => {
  const bills = ref<MonthlyBill[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const currentYearMonth = ref<string>("");
  const summary = ref<BillSummary>({
    total_amount: 0,
    total_paid: 0,
    total_pending: 0,
    bill_count: 0,
    pending_count: 0,
    paid_count: 0,
  });

  const totalCount = computed(() => bills.value.length);

  const billsByStatus = computed(() => {
    const groups: Record<string, MonthlyBill[]> = {};
    for (const b of bills.value) {
      if (!groups[b.status]) groups[b.status] = [];
      groups[b.status]!.push(b);
    }
    return groups;
  });

  const pendingBills = computed(() => billsByStatus.value['待缴费'] || []);
  const paidBills = computed(() => billsByStatus.value['已支付'] || []);
  const partialBills = computed(() => billsByStatus.value['部分支付'] || []);

  const totalPendingAmount = computed(() => summary.value.total_pending);

  const totalPaidAmount = computed(() => summary.value.total_paid);

  async function fetchBills(yearMonth?: string) {
    loading.value = true;
    error.value = null;
    try {
      bills.value = await callCommand<MonthlyBill[]>("list_bills", {
        year_month: yearMonth || null,
      });
      await fetchSummary(yearMonth);
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error("获取账单列表失败:", e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchSummary(yearMonth?: string) {
    try {
      summary.value = await callCommand<BillSummary>("get_bill_summary", {
        year_month: yearMonth || null,
      });
    } catch (e) {
      console.error("获取账单汇总失败:", e);
    }
  }

  async function generateBills(
    req: GenerateBillsRequest
  ): Promise<BillResponse> {
    return callCommand<BillResponse>("generate_monthly_bills", { req });
  }

  function getBillById(id: number): MonthlyBill | undefined {
    return bills.value.find((b) => b.id === id);
  }

  function getBillsByRoomId(roomId: number): MonthlyBill[] {
    return bills.value.filter((b) => b.room_id === roomId);
  }

  return {
    bills,
    loading,
    error,
    currentYearMonth,
    summary,
    totalCount,
    pendingBills,
    paidBills,
    partialBills,
    totalPendingAmount,
    totalPaidAmount,
    fetchBills,
    fetchSummary,
    generateBills,
    getBillById,
    getBillsByRoomId,
  };
});
