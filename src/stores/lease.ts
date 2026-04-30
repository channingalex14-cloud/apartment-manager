/**
 * 合同状态管理
 */

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { callCommand } from "@/services/api";
import type { Lease, LeaseResponse } from "@/types/lease";

export interface CheckInRequest {
  room_id: number;
  tenant_id: number;
  lease_id: number;
  move_in_date: string;
  operator?: string;
}

export interface CheckOutRequest {
  lease_id: number;
  room_id: number;
  move_out_date: string;
  reason: string;
  operator?: string;
}

export const useLeaseStore = defineStore("lease", () => {
  // 状态
  const leases = ref<Lease[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // 计算属性
  const totalCount = computed(() => leases.value.length);

  const activeLeases = computed(
    () => leases.value.filter((l) => l.status === "生效中")
  );

  const pendingSettleLeases = computed(
    () => leases.value.filter((l) => l.status === "待结算")
  );

  const violationLeases = computed(
    () => leases.value.filter((l) => l.status === "违约中")
  );

  // 操作
  async function fetchLeases() {
    loading.value = true;
    error.value = null;
    try {
      leases.value = await callCommand<Lease[]>("list_leases");
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error("获取合同列表失败:", e);
    } finally {
      loading.value = false;
    }
  }

  async function checkIn(req: CheckInRequest): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("check_in", { req });
  }

  async function checkOut(req: CheckOutRequest): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("check_out", { req });
  }

  async function markViolation(leaseId: number): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("mark_violation", { lease_id: leaseId });
  }

  function getLeaseById(id: number): Lease | undefined {
    return leases.value.find((l) => l.id === id);
  }

  function getLeaseByRoomId(roomId: number): Lease | undefined {
    return leases.value.find(
      (l) => l.room_id === roomId && l.status === "生效中"
    );
  }

  return {
    // 状态
    leases,
    loading,
    error,
    // 计算属性
    totalCount,
    activeLeases,
    pendingSettleLeases,
    violationLeases,
    // 操作
    fetchLeases,
    checkIn,
    checkOut,
    markViolation,
    getLeaseById,
    getLeaseByRoomId,
  };
});
