<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { ElMessage } from "element-plus";
import { reportService } from "@/services/report.service";
import { formatMoney, toYuanNumber } from "@/utils/money";

const loading = ref(false);
const generating = ref(false);
const reports = ref<any[]>([]);
const selectedYearMonth = ref("");
const detailDialogVisible = ref(false);
const detailReport = ref<any>(null);

const trendChartRef = ref<HTMLDivElement | null>(null);
const pieChartRef = ref<HTMLDivElement | null>(null);
let trendChart: any = null;
let pieChart: any = null;

const now = new Date();
const currentYear = now.getFullYear();
const yearMonthOptions: { label: string; value: string }[] = [];
for (let y = currentYear; y >= currentYear - 2; y--) {
  for (let m = 12; m >= 1; m--) {
    yearMonthOptions.push({
      label: `${y}年${m.toString().padStart(2, "0")}月`,
      value: `${y}-${m.toString().padStart(2, "0")}`,
    });
  }
}

const recentReports = computed(() => reports.value.slice(0, 12));

onMounted(async () => {
  await fetchReports();
});

onBeforeUnmount(() => {
  trendChart?.dispose();
  pieChart?.dispose();
});

async function fetchReports() {
  loading.value = true;
  try {
    const resp = await reportService.listSummaryReports();
    if (resp.success) {
      reports.value = resp.data || [];
      await initCharts();
    } else {
      ElMessage.error(resp.message || "获取报表失败");
    }
  } catch (e) {
    ElMessage.error("获取报表失败");
  } finally {
    loading.value = false;
  }
}

async function initCharts() {
  if (!trendChartRef.value || !pieChartRef.value) return;

  const [echartsMod] = await Promise.all([
    import('echarts'),
    Promise.resolve(),
  ]);
  const echarts = echartsMod;

  trendChart = echarts.init(trendChartRef.value);
  pieChart = echarts.init(pieChartRef.value);

  const colors = {
    rent: '#409EFF',
    property: '#67C23A',
    water: '#E6A23C',
    electric: '#F56C6C',
    paid: '#67C23A',
    pending: '#F56C6C',
  };

  const formatWanYuan = (v: number) => `¥${(v / 10000).toFixed(1)}万`;

  const sortedReports = [...recentReports.value].reverse().slice(-6);
  const months = sortedReports.map(r => r.yearMonth?.replace('-', '年') + '月' || '');
  const rentData = sortedReports.map(r => toYuanNumber(r.rentTotal || 0));
  const propertyData = sortedReports.map(r => toYuanNumber(r.propertyTotal || 0));
  const waterData = sortedReports.map(r => toYuanNumber((r.waterTotal || 0) + (r.electricTotal || 0)));

  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['房租', '物业费', '水电费'], bottom: 0 },
    grid: { top: 10, bottom: 40, left: 60, right: 20 },
    xAxis: { type: 'category', data: months, axisLabel: { fontSize: 11 } },
    yAxis: { type: 'value', axisLabel: { formatter: formatWanYuan } },
    series: [
      { name: '房租', type: 'bar', data: rentData, itemStyle: { color: colors.rent } },
      { name: '物业费', type: 'bar', data: propertyData, itemStyle: { color: colors.property } },
      { name: '水电费', type: 'bar', data: waterData, itemStyle: { color: colors.water } },
    ],
  }, true);

  const latest = recentReports.value[0];
  if (latest) {
    const paid = (latest.actualPaidTotal || 0);
    const pending = (latest.rentTotal || 0) + (latest.propertyTotal || 0) + (latest.waterTotal || 0) + (latest.electricTotal || 0) - paid;
    const total = paid + pending;
    pieChart.setOption({
      tooltip: { formatter: '{b}: {c}元 ({d}%)' },
      legend: { bottom: 0 },
      series: [{
        type: 'pie',
        radius: ['40%', '65%'],
        center: ['50%', '45%'],
        label: { formatter: '{b}\n{d}%' },
        data: [
          { value: Math.max(0, paid), name: '已收款', itemStyle: { color: colors.paid } },
          { value: Math.max(0, pending), name: '待收款', itemStyle: { color: colors.pending } },
        ],
      }],
    }, true);
  } else {
    pieChart.setOption({
      tooltip: {},
      series: [{
        type: 'pie',
        radius: ['40%', '65%'],
        center: ['50%', '45%'],
        data: [],
      }],
    }, true);
  }
}

async function generateReport() {
  if (!selectedYearMonth.value) {
    ElMessage.warning("请选择年月");
    return;
  }
  generating.value = true;
  try {
    const resp = await reportService.generateMonthlySummary(selectedYearMonth.value);
    if (resp.success) {
      ElMessage.success("报表生成成功");
      await fetchReports();
      selectedYearMonth.value = "";
    } else {
      ElMessage.error(resp.message || "生成失败");
    }
  } catch {
    ElMessage.error("生成失败");
  } finally {
    generating.value = false;
  }
}

async function viewReport(yearMonth: string) {
  loading.value = true;
  try {
    const resp = await reportService.getSummaryReport(yearMonth);
    if (resp.success && resp.data) {
      detailReport.value = resp.data;
      detailDialogVisible.value = true;
    } else {
      ElMessage.error(resp.message || "获取报表详情失败");
    }
  } catch {
    ElMessage.error("获取报表详情失败");
  } finally {
    loading.value = false;
  }
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "-";
  return dateStr.slice(0, 16);
}
</script>

<template>
  <div class="reports">
    <el-card class="tool-card">
      <template #header>
        <span>生成报表</span>
      </template>
      <div class="generate-form">
        <el-select v-model="selectedYearMonth" placeholder="选择年月" style="width: 200px">
          <el-option v-for="opt in yearMonthOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
        </el-select>
        <el-button type="primary" :loading="generating" @click="generateReport" style="margin-left: 12px">
          生成
        </el-button>
      </div>
    </el-card>

    <el-row :gutter="16" class="chart-row" v-loading="loading">
      <el-col :span="16">
        <el-card shadow="hover">
          <template #header>
            <span>收入趋势（近6月）</span>
          </template>
          <div ref="trendChartRef" class="chart-container"></div>
          <div v-if="recentReports.length === 0 && !loading" class="chart-empty">
            <el-empty description="暂无数据" :image-size="60" />
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <span>本月收缴情况</span>
          </template>
          <div ref="pieChartRef" class="chart-container-small"></div>
          <div v-if="recentReports.length === 0 && !loading" class="chart-empty">
            <el-empty description="暂无数据" :image-size="60" />
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card class="report-card" v-loading="loading">
      <template #header>
        <div class="card-header">
          <span>历史报表</span>
        </div>
      </template>

      <el-table :data="reports" stripe>
        <el-table-column prop="yearMonth" label="年月" width="120">
          <template #default="{ row }">
            {{ row.yearMonth?.replace("-", "年") + "月" }}
          </template>
        </el-table-column>
        <el-table-column label="入住率" width="100">
          <template #default="{ row }">
            {{ row.occupancyRate }}%
          </template>
        </el-table-column>
        <el-table-column label="房租收入" width="120">
          <template #default="{ row }">
            {{ formatMoney(row.rentTotal) }}
          </template>
        </el-table-column>
        <el-table-column label="实际收款" width="120">
          <template #default="{ row }">
            {{ formatMoney(row.actualPaidTotal) }}
          </template>
        </el-table-column>
        <el-table-column label="总费用" width="120">
          <template #default="{ row }">
            {{ formatMoney((row.rentTotal || 0) + (row.propertyTotal || 0) + (row.waterTotal || 0) + (row.electricTotal || 0)) }}
          </template>
        </el-table-column>
        <el-table-column prop="generatedAt" label="生成时间" width="180">
          <template #default="{ row }">
            {{ formatDate(row.generatedAt) }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewReport(row.yearMonth)">
              查看详情
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="reports.length === 0 && !loading" class="empty-tip">
        <el-empty description="暂无报表数据，请先生成" />
      </div>
    </el-card>

    <el-dialog v-model="detailDialogVisible" title="报表详情" width="700px">
      <template v-if="detailReport">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="统计年月">{{ detailReport.yearMonth?.replace('-', '年') + '月' }}</el-descriptions-item>
          <el-descriptions-item label="入住率">{{ detailReport.occupancyRate }}%</el-descriptions-item>
          <el-descriptions-item label="总房间">{{ detailReport.totalRooms }}</el-descriptions-item>
          <el-descriptions-item label="在租/空房">{{ detailReport.rentedCount }} / {{ detailReport.vacantCount }}</el-descriptions-item>
          <el-descriptions-item label="违约/新租">{{ detailReport.violationCount }} / {{ detailReport.newRentedCount }}</el-descriptions-item>
          <el-descriptions-item label="员工/管理">{{ detailReport.staffCount }} / {{ detailReport.managementCount }}</el-descriptions-item>
          <el-descriptions-item label="房租合计">{{ formatMoney(detailReport.rentTotal) }}</el-descriptions-item>
          <el-descriptions-item label="物业费合计">{{ formatMoney(detailReport.propertyTotal) }}</el-descriptions-item>
          <el-descriptions-item label="水费合计">{{ formatMoney(detailReport.waterTotal) }}</el-descriptions-item>
          <el-descriptions-item label="电费合计">{{ formatMoney(detailReport.electricTotal) }}</el-descriptions-item>
          <el-descriptions-item label="维修费合计">{{ formatMoney(detailReport.repairTotal) }}</el-descriptions-item>
          <el-descriptions-item label="押金收取">{{ formatMoney(detailReport.depositTotal) }}</el-descriptions-item>
          <el-descriptions-item label="上期欠款">{{ formatMoney(detailReport.previousBalanceTotal) }}</el-descriptions-item>
          <el-descriptions-item label="实际收款">
            <span style="color: #67C23A; font-weight: bold;">{{ formatMoney(detailReport.actualPaidTotal) }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="生成时间" :span="2">{{ formatDate(detailReport.generatedAt) }}</el-descriptions-item>
        </el-descriptions>
      </template>
      <template #footer>
        <el-button @click="detailDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.reports {
  .tool-card { margin-top: 16px; }
  .chart-row { margin-top: 16px; }

  .chart-container {
    height: 280px;
    position: relative;
  }

  .chart-container-small {
    height: 280px;
    position: relative;
  }

  .chart-empty {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .report-card { margin-top: 16px; }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .generate-form {
    display: flex;
    align-items: center;
  }

  .empty-tip { margin-top: 20px; }
}
</style>
