<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoomStore } from "@/stores/room";
import { depositService } from "@/services/deposit.service";
import { formatMoney, toCent } from "@/utils/money";
import type { DepositLedgerResponse } from "@/types/deposit";

const roomStore = useRoomStore();

const ledgerData = ref<DepositLedgerResponse | null>(null);
const loading = ref(false);
const roomFilter = ref<number | null>(null);
const depositDialogVisible = ref(false);
const depositAction = ref<"receive" | "refund">("receive");
const depositForm = ref({ leaseId: 0, amount: 0, notes: "" });

onMounted(async () => {
  loading.value = true;
  try {
    await Promise.all([fetchLedger(), roomStore.fetchRooms()]);
  } finally {
    loading.value = false;
  }
});

async function fetchLedger() {
  loading.value = true;
  try {
    ledgerData.value = await depositService.getLedger(undefined, roomFilter.value || undefined);
  } catch {
    ElMessage.error("获取押金台账失败");
  } finally {
    loading.value = false;
  }
}

function getTransactionTypeTag(type: string): "primary" | "success" | "warning" | "info" | "danger" {
  const map: Record<string, "primary" | "success" | "warning" | "info" | "danger"> = {
    收取: "success",
    部分收取: "warning",
    退还: "info",
    抵扣: "danger",
    没收: "danger",
  };
  return map[type] || "info";
}

function openDepositDialog(action: "receive" | "refund") {
  depositAction.value = action;
  depositForm.value = { leaseId: 0, amount: 0, notes: "" };
  depositDialogVisible.value = true;
}

async function handleDepositSubmit() {
  if (!depositForm.value.leaseId) {
    ElMessage.warning("请输入合同ID");
    return;
  }
  if (depositForm.value.amount <= 0) {
    ElMessage.warning("金额必须大于0");
    return;
  }
  const amountFen = toCent(depositForm.value.amount);
  try {
    if (depositAction.value === "receive") {
      await depositService.receiveDeposit(depositForm.value.leaseId, amountFen, "管理员", depositForm.value.notes || undefined);
      ElMessage.success("押金收取成功");
    } else {
      await ElMessageBox.confirm("确定要退还押金吗？", "确认退还", { type: "warning" });
      await depositService.refundDeposit(depositForm.value.leaseId, amountFen, "管理员", depositForm.value.notes || undefined);
      ElMessage.success("押金退还成功");
    }
    depositDialogVisible.value = false;
    await fetchLedger();
  } catch {
    if (depositAction.value === "receive") {
      ElMessage.error("收取失败");
    }
  }
}
</script>

<template>
  <div class="deposit-ledger">
    <!-- 工具栏 -->
    <el-row :gutter="16" class="toolbar">
      <el-col :span="6">
        <el-select
          v-model="roomFilter"
          placeholder="筛选房间"
          clearable
          filterable
          @change="fetchLedger"
        >
          <el-option
            v-for="room in roomStore.rooms"
            :key="room.id"
            :label="room.room_number"
            :value="room.id"
          />
        </el-select>
      </el-col>
      <el-col :span="18" class="toolbar-buttons">
        <el-button type="success" @click="openDepositDialog('receive')">收取押金</el-button>
        <el-button type="warning" @click="openDepositDialog('refund')">退还押金</el-button>
      </el-col>
    </el-row>

    <!-- 汇总 -->
    <el-row :gutter="16" class="stats-row">
      <el-col :span="12">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">押金余额总计</span>
            <span class="stat-value success">
              {{ ledgerData ? formatMoney(ledgerData.total_balance) : "0.00" }}
            </span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">台账记录数</span>
            <span class="stat-value">
              {{ ledgerData?.records.length || 0 }}
            </span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 台账表格 -->
    <el-table
      :data="ledgerData?.records || []"
      v-loading="loading"
      stripe
      class="ledger-table"
    >
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="room_number" label="房间" width="100" />
      <el-table-column prop="tenant_name" label="租客" width="120">
        <template #default="{ row }">
          {{ row.tenant_name || "-" }}
        </template>
      </el-table-column>
      <el-table-column label="交易类型" width="100">
        <template #default="{ row }">
          <el-tag :type="getTransactionTypeTag(row.transaction_type)" size="small">
            {{ row.transaction_type }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="金额" width="120">
        <template #default="{ row }">
          <span
            :class="{
              'amount-in': row.transaction_type === '收取' || row.transaction_type === '退还',
              'amount-out': row.transaction_type === '抵扣' || row.transaction_type === '没收',
            }"
          >
            {{ row.amount >= 0 ? "+" : "" }}{{ formatMoney(row.amount) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column label="余额" width="120">
        <template #default="{ row }">
          {{ formatMoney(row.balance) }}
        </template>
      </el-table-column>
      <el-table-column label="日期" width="120">
        <template #default="{ row }">
          {{ row.transaction_date || "-" }}
        </template>
      </el-table-column>
      <el-table-column prop="operator" label="操作员" width="100">
        <template #default="{ row }">
          {{ row.operator || "-" }}
        </template>
      </el-table-column>
      <el-table-column prop="notes" label="备注">
        <template #default="{ row }">
          {{ row.notes || "-" }}
        </template>
      </el-table-column>
    </el-table>

    <div v-if="(ledgerData?.records.length || 0) === 0 && !loading" class="empty-tip">
      <el-empty description="暂无台账记录" />
    </div>

    <el-dialog v-model="depositDialogVisible" :title="depositAction === 'receive' ? '收取押金' : '退还押金'" width="450px">
      <el-form :model="depositForm" label-width="100px">
        <el-form-item label="合同ID">
          <el-input-number v-model="depositForm.leaseId" :min="1" :precision="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="金额(元)">
          <el-input-number v-model="depositForm.amount" :min="0.01" :precision="2" :step="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="depositForm.notes" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="depositDialogVisible = false">取消</el-button>
        <el-button :type="depositAction === 'receive' ? 'success' : 'warning'" @click="handleDepositSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.deposit-ledger {
  .toolbar {
    margin-top: 16px;
    margin-bottom: 16px;
    align-items: center;

    .toolbar-buttons {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
    }
  }

  .stats-row {
    margin-bottom: 16px;

    .stat-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px 0;

      .stat-label {
        font-size: 14px;
        color: var(--el-text-color-secondary);
        margin-bottom: 4px;
      }

      .stat-value {
        font-size: 24px;
        font-weight: bold;
        color: var(--el-color-primary);

        &.success { color: var(--el-color-success); }
        &.danger { color: var(--el-color-danger); }
        &.warning { color: var(--el-color-warning); }
      }
    }
  }

  .ledger-table {
    .amount-in {
      color: var(--el-color-success);
      font-weight: bold;
    }

    .amount-out {
      color: var(--el-color-danger);
      font-weight: bold;
    }
  }

  .empty-tip {
    margin-top: 40px;
  }
}
</style>
