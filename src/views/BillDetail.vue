<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import { billService } from "@/services/bill.service";
import { toYuanString, toCent } from "@/utils/money";
import type { BillDetailResponse } from "@/types/bill";

const route = useRoute();
const router = useRouter();

const loading = ref(false);
const bill = ref<BillDetailResponse | null>(null);
const error = ref<string | null>(null);

// 水电用量
const waterUsage = computed(() => {
  if (!bill.value) return 0;
  return bill.value.water_reading_current - bill.value.water_reading_prev;
});

const electricUsage = computed(() => {
  if (!bill.value) return 0;
  return bill.value.electric_reading_current - bill.value.electric_reading_prev;
});

// 状态颜色
function getStatusType(status: string): "primary" | "success" | "warning" | "info" | "danger" {
  const map: Record<string, "primary" | "success" | "warning" | "info" | "danger"> = {
    "待缴费": "warning",
    "已支付": "success",
    "部分支付": "info",
    "已作废": "danger",
  };
  return map[status] || "info";
}

// 加载账单详情
async function loadBillDetail() {
  const id = Number(route.params.id);
  if (isNaN(id)) {
    error.value = "无效的账单ID";
    return;
  }

  loading.value = true;
  error.value = null;
  try {
    bill.value = await billService.getBillDetail(id);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    ElMessage.error("加载账单详情失败: " + error.value);
  } finally {
    loading.value = false;
  }
}

function goBack() {
  router.push({ name: "BillList" });
}

function goPrint() {
  router.push({ name: "BillPrint", params: { id: route.params.id } });
}

// 确认支付
async function handleConfirmPay() {
  if (!bill.value) return;
  try {
    const res = await billService.confirmBillPaid(bill.value.id);
    if (res.success) {
      ElMessage.success(res.message);
      await refreshDetail();
    } else {
      ElMessage.error(res.message);
    }
  } catch (e) {
    ElMessage.error("操作失败: " + (e instanceof Error ? e.message : String(e)));
  }
}

// 部分支付
async function handlePartialPay() {
  if (!bill.value) return;
  try {
    const { value: amountStr } = await ElMessageBox.prompt(
      `剩余金额 ¥${toYuanString(bill.value.remaining_amount)}，请输入支付金额（元）`,
      "部分支付",
      {
        confirmButtonText: "确认支付",
        cancelButtonText: "取消",
        inputPattern: /^\d+(\.\d{1,2})?$/,
        inputErrorMessage: "请输入有效金额",
      }
    );
    const amountFen = toCent(amountStr);
    const res = await billService.partialPayBill(bill.value.id, amountFen);
    if (res.success) {
      ElMessage.success(res.message);
      await refreshDetail();
    } else {
      ElMessage.error(res.message);
    }
  } catch (e) {
    // user cancelled
  }
}

// 作废账单
async function handleVoid() {
  if (!bill.value) return;
  try {
    await ElMessageBox.confirm("确认作废此账单？作废后不可恢复。", "警告", {
      confirmButtonText: "确认作废",
      cancelButtonText: "取消",
      type: "warning",
    });
    const res = await billService.voidBill(bill.value.id);
    if (res.success) {
      ElMessage.success(res.message);
      await refreshDetail();
    } else {
      ElMessage.error(res.message);
    }
  } catch (e) {
    // user cancelled
  }
}

// 终态判断
const isTerminal = computed(() => {
  return bill.value?.status === "已支付" || bill.value?.status === "已作废";
});

// 费用明细表格数据
interface FeeRow {
  name: string;
  amount: string;
  note: string;
  highlight: boolean;
}

const feeTableData = computed<FeeRow[]>(() => {
  if (!bill.value) return [];
  return [
    { name: "租金", amount: `¥${toYuanString(bill.value.rent_fee)}`, note: "", highlight: true },
    { name: "物业费", amount: `¥${toYuanString(bill.value.property_fee)}`, note: "", highlight: false },
    { name: "水费", amount: `¥${toYuanString(bill.value.water_fee)}`, note: `${waterUsage.value}吨`, highlight: false },
    { name: "电费", amount: `¥${toYuanString(bill.value.electric_fee)}`, note: `${electricUsage.value}度`, highlight: false },
    { name: "管理费", amount: `¥${toYuanString(bill.value.management_fee)}`, note: "", highlight: false },
    { name: "维修费", amount: `¥${toYuanString(bill.value.repair_fee)}`, note: "", highlight: false },
    { name: "杂费", amount: `¥${toYuanString(bill.value.misc_fee)}`, note: bill.value.misc_fee_remark || "", highlight: false },
    { name: "上期余额", amount: `¥${toYuanString(bill.value.previous_balance)}`, note: "", highlight: false },
  ];
});

async function refreshDetail() {
  if (!bill.value) return;
  bill.value = await billService.getBillDetail(bill.value.id);
}

onMounted(() => {
  loadBillDetail();
});
</script>

<template>
  <div class="bill-detail" v-loading="loading">
    <!-- 返回 -->
    <el-button text @click="goBack">← 返回列表</el-button>

    <div v-if="error" class="error-tip">
      <el-alert type="error" :title="error" show-icon :closable="false" />
    </div>

    <div v-if="bill" class="bill-content">
      <!-- 基本信息 -->
      <el-card class="info-card" shadow="never">
        <template #header>
          <span class="card-title">基本信息</span>
        </template>
        <el-descriptions :column="2" border>
          <el-descriptions-item label="房间">
            {{ bill.building }}{{ bill.room_number }}
          </el-descriptions-item>
          <el-descriptions-item label="租客">
            {{ bill.tenant_name || "-" }}
            <span v-if="bill.tenant_phone" class="phone">
              {{ bill.tenant_phone }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="账单月份">
            {{ bill.year_month }}
          </el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="getStatusType(bill.status)">{{ bill.status }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="到期日">
            {{ bill.due_date || "-" }}
          </el-descriptions-item>
          <el-descriptions-item label="生成时间">
            {{ bill.created_at || "-" }}
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- 费用明细 -->
      <el-card class="fee-card" shadow="never">
        <template #header>
          <span class="card-title">费用明细</span>
        </template>
        <el-table :data="feeTableData" stripe size="small">
          <el-table-column prop="name" label="项目" width="160" />
          <el-table-column prop="amount" label="金额(元)" align="right">
            <template #default="{ row }">
              <span :class="{ highlight: row.highlight }">{{ row.amount }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="note" label="说明" />
        </el-table>

        <div class="summary-section">
          <el-row :gutter="12">
            <el-col :span="8">
              <div class="summary-item">
                <span class="label">合计</span>
                <span class="value primary">¥{{ toYuanString(bill.total_amount) }}</span>
              </div>
            </el-col>
            <el-col :span="8">
              <div class="summary-item">
                <span class="label">已付</span>
                <span class="value success">¥{{ toYuanString(bill.actual_paid) }}</span>
              </div>
            </el-col>
            <el-col :span="8">
              <div class="summary-item">
                <span class="label">剩余</span>
                <span class="value warning">¥{{ toYuanString(bill.remaining_amount) }}</span>
              </div>
            </el-col>
          </el-row>
        </div>
      </el-card>

      <!-- 水电读数 -->
      <el-card class="meter-card" shadow="never">
        <template #header>
          <span class="card-title">水电读数</span>
        </template>
        <el-descriptions :column="1" border>
          <el-descriptions-item label="水表">
            上期 {{ bill.water_reading_prev }} → 本期 {{ bill.water_reading_current }}
            （用量 {{ waterUsage }} 吨）
          </el-descriptions-item>
          <el-descriptions-item label="电表">
            上期 {{ bill.electric_reading_prev }} → 本期 {{ bill.electric_reading_current }}
            （用量 {{ electricUsage }} 度）
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- 操作按钮（仅非终态显示） -->
      <el-card v-if="!isTerminal" class="action-card" shadow="never">
        <div class="action-buttons">
          <el-button type="info" size="large" @click="goPrint">
            打印通知单
          </el-button>
          <el-button
            v-if="bill.can_confirm"
            type="success"
            size="large"
            @click="handleConfirmPay"
          >
            确认支付
          </el-button>
          <el-button
            v-if="bill.can_pay_partial"
            type="primary"
            size="large"
            @click="handlePartialPay"
          >
            部分支付
          </el-button>
          <el-button
            v-if="bill.can_void"
            type="danger"
            size="large"
            @click="handleVoid"
          >
            作废账单
          </el-button>
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped lang="scss">
.bill-detail {
  .error-tip {
    margin: 16px 0;
  }

  .card-title {
    font-weight: 600;
    font-size: 15px;
  }

  .info-card,
  .fee-card,
  .meter-card,
  .action-card {
    margin-top: 16px;
  }

  .phone {
    color: var(--el-text-color-secondary);
    margin-left: 8px;
    font-size: 13px;
  }

  .highlight {
    font-weight: 600;
    color: var(--el-text-color-primary);
  }

  .summary-section {
    margin-top: 16px;
    padding: 12px 16px;
    background: var(--el-bg-color-page);
    border-radius: 4px;

    .summary-item {
      display: flex;
      flex-direction: column;
      align-items: center;

      .label {
        font-size: 13px;
        color: var(--el-text-color-secondary);
        margin-bottom: 4px;
      }

      .value {
        font-size: 22px;
        font-weight: bold;

        &.primary { color: var(--el-color-primary); }
        &.success { color: var(--el-color-success); }
        &.warning { color: var(--el-color-warning); }
      }
    }
  }

  .action-card {
    .action-buttons {
      display: flex;
      gap: 12px;
      justify-content: center;
    }
  }
}
</style>