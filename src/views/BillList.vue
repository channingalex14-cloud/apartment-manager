<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { useRoomStore } from "@/stores/room";
import { useBillStore } from "@/stores/bill";
import { useUIStore } from "@/stores/ui";
import { formatMoney } from "@/utils/money";

const router = useRouter();
const roomStore = useRoomStore();
const billStore = useBillStore();
const uiStore = useUIStore();

const loading = ref(false);
const statusFilter = ref("");
const yearMonthFilter = ref("");

onMounted(async () => {
  loading.value = true;
  try {
    await Promise.all([billStore.fetchBills(), roomStore.fetchRooms()]);
  } finally {
    loading.value = false;
  }
});

const statusOptions = [
  { label: "全部状态", value: "" },
  { label: "待缴费", value: "待缴费" },
  { label: "已支付", value: "已支付" },
  { label: "部分支付", value: "部分支付" },
  { label: "已作废", value: "已作废" },
];

const yearMonthOptions = computed(() => {
  const months = new Set(billStore.bills.map(b => b.year_month).filter(Boolean));
  return [{ label: "全部月份", value: "" }, ...[...months].sort().reverse().map(v => ({ label: v, value: v }))];
});

const filteredBills = computed(() => {
  let result = billStore.bills;
  if (statusFilter.value) {
    result = result.filter(b => b.status === statusFilter.value);
  }
  if (yearMonthFilter.value) {
    result = result.filter(b => b.year_month === yearMonthFilter.value);
  }
  const kw = uiStore.globalSearchKeyword.trim().toLowerCase();
  if (!kw) return result;
  return result.filter(b => {
    const room = roomStore.getRoomById(b.room_id);
    return (
      room?.room_number.toLowerCase().includes(kw) ||
      room?.tenant_name?.toLowerCase().includes(kw)
    );
  });
});

const statusCounts = computed(() => {
  const counts: Record<string, number> = {};
  for (const b of billStore.bills) {
    counts[b.status] = (counts[b.status] || 0) + 1;
  }
  return counts;
});

function getRoomNumber(roomId: number): string {
  return roomStore.getRoomById(roomId)?.room_number || String(roomId);
}

function getStatusTagType(status: string): "primary" | "success" | "warning" | "danger" | "info" {
  const map: Record<string, "primary" | "success" | "warning" | "danger" | "info"> = {
    "待缴费": "warning",
    "已支付": "success",
    "部分支付": "info",
    "已作废": "danger",
  };
  return map[status] || "info";
}

function viewBillDetail(bill: any) {
  router.push(`/bills/${bill.id}`);
}
</script>

<template>
  <div class="bill-list">
    <div class="toolbar">
      <div class="filters">
        <el-select v-model="statusFilter" placeholder="账单状态" clearable style="width: 120px; margin-right: 8px">
          <el-option v-for="opt in statusOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
        </el-select>
        <el-select v-model="yearMonthFilter" placeholder="选择月份" clearable style="width: 120px">
          <el-option v-for="opt in yearMonthOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
        </el-select>
      </div>
      <div class="status-counts">
        <span v-for="(count, status) in statusCounts" :key="status" class="status-count" :class="`status-${status}`">
          {{ status }}: {{ count }}
        </span>
      </div>
    </div>

    <el-table :data="filteredBills" v-loading="loading" stripe>
      <el-table-column label="房间" width="90">
        <template #default="{ row }">{{ getRoomNumber(row.room_id) }}</template>
      </el-table-column>
      <el-table-column prop="year_month" label="月份" width="100" />
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusTagType(row.status)" size="small">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="水费" width="80" align="right">
        <template #default="{ row }">{{ formatMoney(row.water_fee) }}</template>
      </el-table-column>
      <el-table-column label="电费" width="80" align="right">
        <template #default="{ row }">{{ formatMoney(row.electric_fee) }}</template>
      </el-table-column>
      <el-table-column label="物业费" width="80" align="right">
        <template #default="{ row }">{{ formatMoney(row.property_fee) }}</template>
      </el-table-column>
      <el-table-column label="租金" width="90" align="right">
        <template #default="{ row }">{{ formatMoney(row.rent_fee) }}</template>
      </el-table-column>
      <el-table-column label="总额" width="100" align="right">
        <template #default="{ row }">
          <span class="total-amount">{{ formatMoney(row.total_amount) }}</span>
        </template>
      </el-table-column>
      <el-table-column label="实付" width="90" align="right">
        <template #default="{ row }">
          <span class="paid-amount">{{ formatMoney(row.actual_paid) }}</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="80">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="viewBillDetail(row)">详情</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="filteredBills.length === 0 && !loading" class="empty-tip">
      <el-empty description="暂无账单数据" />
    </div>
  </div>
</template>

<style scoped lang="scss">
.bill-list {
  padding: 16px;

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;

    .filters {
      display: flex;
      align-items: center;
    }

    .status-counts {
      display: flex;
      gap: 12px;
      font-size: 13px;

      .status-待缴费 { color: var(--el-color-warning); }
      .status-已支付 { color: var(--el-color-success); }
      .status-部分支付 { color: var(--el-color-info); }
      .status-已作废 { color: var(--el-color-danger); }
    }
  }

  .total-amount {
    font-weight: bold;
    color: var(--el-color-primary);
  }

  .paid-amount {
    color: var(--el-color-success);
  }

  .empty-tip { margin-top: 40px; }
}
</style>
