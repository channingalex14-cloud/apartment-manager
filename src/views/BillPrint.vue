<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { billService } from "@/services/bill.service";
import { configService } from "@/services/config.service";
import { formatMoneyInt, toYuanString } from "@/utils/money";
import type { BillDetailResponse } from "@/types/bill";
import type { SystemConfig } from "@/types/config";

const route = useRoute();
const router = useRouter();

const loading = ref(false);
const bill = ref<BillDetailResponse | null>(null);
const configs = ref<SystemConfig[]>([]);
const error = ref<string | null>(null);

// 风格选择：3=极简商务  5=现代卡片  6=高端收据
const printStyle = ref(5);

const waterUsage = computed(() => {
  if (!bill.value) return 0;
  return bill.value.water_reading_current - bill.value.water_reading_prev;
});
const electricUsage = computed(() => {
  if (!bill.value) return 0;
  return bill.value.electric_reading_current - bill.value.electric_reading_prev;
});

function getConfig(key: string): string {
  return configs.value.find(c => c.config_key === key)?.config_value || "";
}

const apartmentName = computed(() => getConfig("公寓名称") || "新逸公寓");
const buildingAddress = computed(() => getConfig("楼栋地址") || "58栋");
const tips = computed(() => getConfig("温馨提示") || "请及时缴纳房租水电。逾期收取滞纳金。");
const tipsDisplay = computed(() => {
  const base = tips.value.replace(/转账记录请发截图登记[，,]?/g, "").replace(/^[,，。.\s]+|[,，。.\s]+$/g, "").trim() || "请及时缴纳房租水电。逾期收取滞纳金";
  return base.endsWith("。") ? base : base + "。";
});

const qrImageUrl = computed(() => {
  const filename = getConfig("收款二维码");
  if (!filename) return "";
  return `/bill-assets/${filename}`;
});

const yearMonthStr = computed(() => {
  if (!bill.value) return "";
  const [year, month] = bill.value.year_month.split("-");
  return `${year}年${parseInt(month || '1')}月`;
});

const totalAmount = computed(() => {
  if (!bill.value) return "0";
  return formatMoneyInt(bill.value.total_amount);
});

const waterUnitPrice = computed(() => toYuanString(Number(getConfig("水费单价"))));
const electricUnitPrice = computed(() => toYuanString(Number(getConfig("电费单价"))));
const managementUnitPrice = computed(() => toYuanString(Number(getConfig("管理费单价"))));

async function loadData() {
  const id = Number(route.params.id);
  if (isNaN(id)) { error.value = "无效的账单ID"; return; }
  loading.value = true;
  error.value = null;
  try {
    const [billData, configData] = await Promise.all([
      billService.getBillDetail(id),
      configService.getAll(),
    ]);
    bill.value = billData;
    configs.value = configData;
  } catch (e: any) {
    error.value = e?.message || String(e);
    ElMessage.error("加载失败: " + error.value);
  } finally {
    loading.value = false;
  }
}

function goBack() { router.push({ name: "BillDetail", params: { id: route.params.id } }); }
function doPrint() { window.print(); }

onMounted(() => { loadData(); });
</script>

<template>
  <div class="bill-print-page" v-loading="loading">

    <!-- 操作栏 -->
    <div class="print-toolbar no-print" v-if="!loading && bill">
      <el-button @click="goBack">← 返回</el-button>
      <el-button type="primary" @click="doPrint">🖨 打印</el-button>
    </div>

    <div v-if="error" class="error-tip">
      <el-alert type="error" :title="error" show-icon :closable="false" />
    </div>

    <!-- ═══════════════════════════════════════════════════════════════
         风格 5：现代卡片风 · 科技快递面单感
         蓝橙撞色，左费用卡右二维码
         ═══════════════════════════════════════════════════════════════ -->
    <div v-if="bill && printStyle === 5" class="print-area style-5" id="bill-print-area">
      <div class="s5-card">

        <!-- 顶部标题 -->
        <div class="s5-top">
          <div class="s5-title-group">
            <div class="s5-title">新逸公寓收费通知单</div>
            <div class="s5-room-tag">
              <span>{{ buildingAddress }}</span>
              <span class="s5-room-div">|</span>
              <span>{{ bill.room_number }} 室</span>
              <span class="s5-room-div">|</span>
              <span>{{ yearMonthStr }}</span>
            </div>
          </div>
        </div>

        <!-- 主体内容 -->
        <div class="s5-main">
          <!-- 左侧费用表 -->
          <div class="s5-table-area">
            <table class="s5-table">
              <thead>
                <tr>
                  <th class="s5-th-item">费用项目</th>
                  <th class="s5-th-num">本月读数</th>
                  <th class="s5-th-num">上月读数</th>
                  <th class="s5-th-num">用量</th>
                  <th class="s5-th-num">单价</th>
                  <th class="s5-th-amt">应付金额</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td class="s5-item">水费</td>
                  <td class="s5-n">{{ bill.water_reading_current }}</td>
                  <td class="s5-n">{{ bill.water_reading_prev }}</td>
                  <td class="s5-n">{{ waterUsage }} 吨</td>
                  <td class="s5-n">{{ waterUnitPrice }}/吨</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.water_fee) }}</td>
                </tr>
                <tr>
                  <td class="s5-item">电费</td>
                  <td class="s5-n">{{ bill.electric_reading_current }}</td>
                  <td class="s5-n">{{ bill.electric_reading_prev }}</td>
                  <td class="s5-n">{{ electricUsage }} 度</td>
                  <td class="s5-n">{{ electricUnitPrice }}/度</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.electric_fee) }}</td>
                </tr>
                <tr>
                  <td class="s5-item">管理费</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">{{ managementUnitPrice }}/月</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.management_fee) }}</td>
                </tr>
                <tr>
                  <td class="s5-item">房租</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.rent_fee) }}</td>
                </tr>
                <tr>
                  <td class="s5-item">物业费</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.property_fee) }}</td>
                </tr>
                <tr>
                  <td class="s5-item">维修费</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.repair_fee) }}</td>
                </tr>
                <tr v-if="bill.misc_fee > 0">
                  <td class="s5-item">{{ bill.misc_fee_remark || "杂费" }}</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n">—</td>
                  <td class="s5-n s5-amt">¥ {{ formatMoneyInt(bill.misc_fee) }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- 右侧二维码卡 -->
          <div v-if="qrImageUrl" class="s5-qr-col">
            <div class="s5-qr-label">转账记录请发截图登记，谢谢合作。</div>
            <div class="s5-qr-box">
              <img :src="qrImageUrl" alt="收款二维码" />
            </div>
          </div>
        </div>

        <!-- 合计栏 -->
        <div class="s5-total-bar">
          <div class="s5-tips">{{ tipsDisplay }}</div>
          <div class="s5-total-right">
            <span class="s5-total-label">合计</span>
            <span class="s5-total-amount">¥ {{ totalAmount }}</span>
          </div>
        </div>

      </div>
    </div>

    <div v-if="!loading && !bill && !error" class="empty-tip">
      <el-empty description="账单不存在" />
    </div>
  </div>
</template>

<style scoped lang="scss">
/* ================================================================
   全局通用
   ================================================================ */
@media print {
  .no-print { display: none !important; }
  .bill-print-page { padding: 0; }
}

.print-toolbar {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.style-label {
  font-size: 13px;
  color: #666;
}

.error-tip { margin: 16px 0; }
.empty-tip { margin-top: 40px; }

/* ================================================================
   打印区域公共
   ================================================================ */
.print-area {
  margin: 0 auto;
  box-sizing: border-box;
  font-family: "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Helvetica, sans-serif;
}

/* ================================================================
   风格 5：现代卡片风 · 科技快递面单感
   蓝橙撞色，左费用卡右二维码
   ================================================================ */
.style-5 {
  /* 屏幕预览：填满可用宽度 */
  width: 100%;
  min-height: 637px;
  padding: 14px 18px;
  background: #EEF2F7;
}

@media print {
  .style-5 {
    width: 297mm;
    min-height: 210mm;
  }
}

.s5-card {
  background: #fff;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 6px 24px rgba(30,60,120,0.10);
}

/* 顶部 */
.s5-top {
  background: linear-gradient(135deg, #2C4A7C 0%, #3A5F9C 50%, #2C4A7C 100%);
  padding: 16px 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.s5-title-group { text-align: center; }

.s5-title {
  font-size: 28px;
  font-weight: 900;
  color: #fff;
  letter-spacing: 8px;
}

.s5-room-tag {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 4px;
  font-size: 18px;
  color: rgba(255,255,255,0.85);
  font-weight: 600;
}

.s5-room-div { color: rgba(255,255,255,0.4); }

/* 主体 */
.s5-main {
  display: flex;
  min-height: 300px;
}

/* 费用表格 */
.s5-table-area {
  flex: 1;
  padding: 14px 16px;
}

.s5-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.s5-table thead tr {
  background: linear-gradient(135deg, #2C4A7C, #3A5F9C);
}

.s5-table th {
  color: #fff;
  font-weight: 700;
  padding: 8px 10px;
  text-align: center;
  font-size: 16px;
  letter-spacing: 0.5px;
}

.s5-th-item {
  text-align: left !important;
  border-radius: 6px 0 0 6px;
}

.s5-th-amt {
  text-align: right !important;
  border-radius: 0 6px 6px 0;
}

.s5-table tbody tr {
  border-bottom: 1px solid #E8EFF8;
  transition: background 0.15s;
}

.s5-table tbody tr:nth-child(even) {
  background: #F5F8FC;
}

.s5-table tbody tr:hover {
  background: #EBF2FA;
}

.s5-item {
  padding: 9px 10px;
  font-weight: 700;
  color: #2C4A7C;
  font-size: 16px;
}

.s5-n {
  padding: 9px 10px;
  text-align: center;
  color: #555;
  font-family: "SF Mono", "Menlo", "Consolas", monospace;
  font-size: 16px;
}

.s5-amt {
  text-align: left !important;
  font-weight: 900;
  color: #FF6B35;
  font-size: 16px;
  padding-left: 8px !important;
}

/* 右侧二维码 */
.s5-qr-col {
  width: 340px;
  flex-shrink: 0;
  background: linear-gradient(180deg, #F5F8FC, #EEF2F7);
  border-left: 3px solid #DDE5F0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px 16px;
  gap: 12px;
}

.s5-qr-label {
  font-size: 18px;
  font-weight: 800;
  color: #2C4A7C;
  letter-spacing: 3px;
}

.s5-qr-box {
  width: 300px;
  height: 300px;
  background: #fff;
  border: 2px solid #DDE5F0;
  border-radius: 12px;
  padding: 10px;
  box-shadow: 0 4px 12px rgba(30,60,120,0.08);

  img {
    width: 280px;
    height: 280px;
    object-fit: contain;
  }
}

/* 合计栏 */
.s5-total-bar {
  display: flex;
  align-items: center;
  border-top: 3px solid #2C4A7C;
  background: #2C4A7C;
}

.s5-tips {
  flex: 1;
  padding: 14px 20px;
  font-size: 16px;
  color: rgba(255,255,255,0.75);
  font-weight: 500;
}

.s5-total-right {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
}

.s5-total-label {
  font-size: 28px;
  color: rgba(255,255,255,0.7);
  font-weight: 700;
  letter-spacing: 1px;
}

.s5-total-amount {
  font-size: 28px;
  font-weight: 900;
  color: #FF6B35;
  letter-spacing: 1px;
  text-shadow: 0 2px 4px rgba(0,0,0,0.2);
}

</style>
