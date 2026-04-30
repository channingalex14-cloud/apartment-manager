<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoomStore } from "@/stores/room";
import { useUIStore } from "@/stores/ui";
import { paymentService } from "@/services/payment.service";
import { formatMoney } from "@/utils/money";
import type { Payment } from "@/types/payment";

const roomStore = useRoomStore();
const uiStore = useUIStore();

const loading = ref(false);
const payments = ref<Payment[]>([]);
const statusFilter = ref("");
const voidDialogVisible = ref(false);
const methodDialogVisible = ref(false);
const actionPaymentId = ref(0);
const newPaymentMethod = ref("");

async function fetchPayments() {
  loading.value = true;
  try {
    payments.value = await paymentService.list();
  } catch {
    ElMessage.error("获取缴费记录失败");
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  loading.value = true;
  try {
    await Promise.all([fetchPayments(), roomStore.fetchRooms()]);
  } finally {
    loading.value = false;
  }
});

const filteredPayments = computed(() => {
  let result = payments.value;
  if (statusFilter.value) {
    result = result.filter(p => p.payment_method === statusFilter.value);
  }
  const kw = uiStore.globalSearchKeyword.trim().toLowerCase();
  if (!kw) return result;
  return result.filter(p => {
    const room = roomStore.getRoomById(p.room_id);
    return (
      room?.room_number.toLowerCase().includes(kw) ||
      room?.tenant_name?.toLowerCase().includes(kw)
    );
  });
});

const paymentMethodOptions = computed(() => {
  const methods = new Set(payments.value.map(p => p.payment_method).filter(Boolean));
  return ["微信", "支付宝", "银行转账", "现金", "其他"].filter(m => methods.has(m));
});

function getRoomNumber(roomId: number): string {
  return roomStore.getRoomById(roomId)?.room_number || String(roomId);
}

function openVoidDialog(payment: Payment) {
  actionPaymentId.value = payment.id;
  voidDialogVisible.value = true;
}

async function handleVoid() {
  try {
    await ElMessageBox.confirm("确定要作废此缴费记录吗？作废后关联账单的已付金额将被回退。", "确认作废", { type: "warning" });
    const result = await paymentService.void(actionPaymentId.value);
    if (result.success) {
      ElMessage.success("已作废");
      voidDialogVisible.value = false;
      await fetchPayments();
    } else {
      ElMessage.error(result.message || "作废失败");
    }
  } catch {
    // cancelled
  }
}

function openMethodDialog(payment: Payment) {
  actionPaymentId.value = payment.id;
  newPaymentMethod.value = payment.payment_method || "";
  methodDialogVisible.value = true;
}

async function handleUpdateMethod() {
  if (!newPaymentMethod.value) {
    ElMessage.warning("请选择付款方式");
    return;
  }
  try {
    const result = await paymentService.updateMethod(actionPaymentId.value, newPaymentMethod.value);
    if (result.success) {
      ElMessage.success("已更新付款方式");
      methodDialogVisible.value = false;
      await fetchPayments();
    } else {
      ElMessage.error(result.message || "更新失败");
    }
  } catch {
    ElMessage.error("更新失败");
  }
}
</script>

<template>
  <div class="payment-list">
    <div class="toolbar">
      <div class="filters">
        <el-select v-model="statusFilter" placeholder="付款方式" clearable style="width: 140px">
          <el-option label="微信" value="微信" />
          <el-option label="支付宝" value="支付宝" />
          <el-option label="银行转账" value="银行转账" />
          <el-option label="现金" value="现金" />
          <el-option label="其他" value="其他" />
        </el-select>
      </div>
    </div>

    <el-table :data="filteredPayments" stripe v-loading="loading">
      <el-table-column prop="id" label="ID" width="60" align="center" />
      <el-table-column label="房间" width="100" align="center">
        <template #default="{ row }">{{ getRoomNumber(row.room_id) }}</template>
      </el-table-column>
      <el-table-column label="姓名" width="100" align="center">
        <template #default="{ row }">{{ roomStore.getRoomById(row.room_id)?.tenant_name || "-" }}</template>
      </el-table-column>
      <el-table-column label="金额" width="120" align="center">
        <template #default="{ row }">
          <span class="amount">{{ formatMoney(row.amount) }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="payment_method" label="付款方式" width="120" align="center" />
      <el-table-column prop="payment_date" label="缴费日期" width="120" align="center" />
      <el-table-column label="付款人" width="100" align="center">
        <template #default="{ row }">{{ row.payer_name || "-" }}</template>
      </el-table-column>
      <el-table-column prop="notes" label="备注" show-overflow-tooltip align="center" />
      <el-table-column label="操作" width="140" fixed="right">
        <template #default="{ row }">
          <el-button type="warning" link size="small" @click="openVoidDialog(row)">作废</el-button>
          <el-button type="primary" link size="small" @click="openMethodDialog(row)">改方式</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="filteredPayments.length === 0 && !loading" class="empty-tip">
      <el-empty description="暂无缴费记录" />
    </div>

    <el-dialog v-model="voidDialogVisible" title="作废缴费" width="400px">
      <p>确定要作废此缴费记录吗？关联账单的已付金额将被回退。</p>
      <template #footer>
        <el-button @click="voidDialogVisible = false">取消</el-button>
        <el-button type="danger" @click="handleVoid">确认作废</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="methodDialogVisible" title="修改付款方式" width="400px">
      <el-form label-width="100px">
        <el-form-item label="付款方式">
          <el-select v-model="newPaymentMethod" placeholder="选择付款方式">
            <el-option label="微信" value="微信" />
            <el-option label="支付宝" value="支付宝" />
            <el-option label="银行转账" value="银行转账" />
            <el-option label="现金" value="现金" />
            <el-option label="其他" value="其他" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="methodDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleUpdateMethod">确认</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.payment-list {
  padding: 16px;
  .toolbar {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    margin-bottom: 16px;
  }
  .amount {
    font-weight: bold;
    color: var(--el-color-primary);
  }
  .empty-tip { margin-top: 40px; }
}
</style>
