/**
 * 租客状态管理
 */

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { callCommand } from "@/services/api";
import type { TenantResponse } from "@/types/tenant";

export interface CreateTenantRequest {
  name: string;
  phone: string;
  phone2?: string;
  emergency_contact?: string;
  emergency_phone?: string;
}

export const useTenantStore = defineStore("tenant", () => {
  // 状态
  const tenants = ref<TenantResponse[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // 计算属性
  const totalCount = computed(() => tenants.value.length);

  // 操作
  async function fetchTenants() {
    loading.value = true;
    error.value = null;
    try {
      tenants.value = await callCommand<TenantResponse[]>("list_tenants");
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error("获取租客列表失败:", e);
    } finally {
      loading.value = false;
    }
  }

  async function createTenant(req: CreateTenantRequest): Promise<number> {
    const id = await callCommand<number>("create_tenant", { req });
    await fetchTenants();
    return id;
  }

  function getTenantById(id: number): TenantResponse | undefined {
    return tenants.value.find((t) => t.id === id);
  }

  return {
    // 状态
    tenants,
    loading,
    error,
    // 计算属性
    totalCount,
    // 操作
    fetchTenants,
    createTenant,
    getTenantById,
  };
});
