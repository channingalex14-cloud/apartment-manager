<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useRoomStore } from '@/stores/room'
import { useBillStore } from '@/stores/bill'
import { useUIStore } from '@/stores/ui'
import { formatMoney, toYuanNumber } from '@/utils/money'

const router = useRouter()
const roomStore = useRoomStore()
const billStore = useBillStore()
const uiStore = useUIStore()

const progressChartRef = ref<HTMLElement>()
const roomStatusChartRef = ref<HTMLElement>()
const trendChartRef = ref<HTMLElement>()
let progressChart: import('echarts').ECharts | null = null
let roomStatusChart: import('echarts').ECharts | null = null
let trendChart: import('echarts').ECharts | null = null
// 组件是否已挂载（用于防止卸载后状态更新）
let isMounted = true

// 收租进度
const totalReceivable = computed(() => billStore.summary.total_amount)
const totalCollected = computed(() => billStore.summary.total_paid)
const collectionRate = computed(() => {
  if (!totalReceivable.value) return 0
  return Math.round((totalCollected.value / totalReceivable.value) * 100)
})
const unpaidAmount = computed(() => billStore.summary.total_pending)

// 房间状态分布
const roomStatusData = computed(() => {
  const statuses = ['在租', '空房', '员工', '管理', '违约', '新租', '待清洁', '维修中']
  const colors: Record<string, string> = {
    '在租': '#67C23A',
    '空房': '#E6A23C',
    '员工': '#409EFF',
    '管理': '#909399',
    '违约': '#F56C6C',
    '新租': '#E6A23C',
    '待清洁': '#D463C8',
    '维修中': '#E6A23C',
  }
  return statuses
    .map(s => ({ name: s, value: roomStore.rooms.filter(r => r.status === s).length }))
    .filter(d => d.value > 0)
    .map(d => ({ ...d, itemStyle: { color: colors[d.name] || '#909399' } }))
})

// 统计卡片数据
const stats = computed(() => [
  { title: '房间总数', value: roomStore.totalCount, suffix: '间' },
  { title: '在租房间', value: roomStore.rentedCount, suffix: '间', color: 'var(--color-success)' },
  { title: '空房', value: roomStore.vacantCount, suffix: '间', color: 'var(--color-warning)' },
  { title: '本月应收', value: toYuanNumber(totalReceivable.value), prefix: '¥', color: 'var(--color-primary)' },
])

// 待缴费账单
const pendingBills = computed(() => billStore.pendingBills.slice(0, 5))

// 当前日期
const currentDate = computed(() => {
  const now = new Date()
  return `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日`
})

/** 获取当前主题的图表颜色 */
function getChartColors() {
  const style = getComputedStyle(document.documentElement)
  return {
    collected: style.getPropertyValue('--donut-collected').trim() || '#008F7A',
    unpaid: style.getPropertyValue('--donut-unpaid').trim() || '#FF9671',
    linePrimary: style.getPropertyValue('--chart-line-primary').trim() || '#2C73D2',
    linePrimaryArea: style.getPropertyValue('--chart-line-primary-area').trim() || 'rgba(44, 115, 210, 0.1)',
    lineSuccess: style.getPropertyValue('--chart-line-success').trim() || '#008F7A',
    lineSuccessArea: style.getPropertyValue('--chart-line-success-area').trim() || 'rgba(0, 143, 122, 0.1)',
    lineWarning: style.getPropertyValue('--chart-line-warning').trim() || '#FF9671',
    lineWarningArea: style.getPropertyValue('--chart-line-warning-area').trim() || 'rgba(255, 150, 113, 0.1)',
  }
}

// 初始化环形图
async function initProgressChart() {
  if (!progressChartRef.value) return
  const echarts = await import('echarts')
  progressChart = echarts.init(progressChartRef.value)
  const colors = getChartColors()

  progressChart.setOption({
    tooltip: {
      trigger: 'item',
      formatter: (params: any) => `${params.name}: ${formatMoney(params.value as number)}`,
    },
    legend: { show: false },
    series: [{
      type: 'pie',
      radius: ['50%', '70%'],
      avoidLabelOverlap: false,
      itemStyle: {
        borderRadius: 8,
        borderColor: '#fff',
        borderWidth: 2,
      },
      label: {
        show: true,
        position: 'center',
        formatter: () => `${collectionRate.value}%\n收租率`,
        fontSize: 20,
        fontWeight: 'bold',
        lineHeight: 28,
      },
      data: [
        { value: totalCollected.value, name: '已收', itemStyle: { color: colors.collected } },
        { value: Math.max(unpaidAmount.value, 0), name: '未收', itemStyle: { color: colors.unpaid } },
      ],
    }],
  })
}

// 初始化房间状态分布饼图
async function initRoomStatusChart() {
  if (!roomStatusChartRef.value) return
  const echarts = await import('echarts')
  roomStatusChart = echarts.init(roomStatusChartRef.value)

  roomStatusChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c}间 ({d}%)' },
    legend: { bottom: 0, type: 'scroll' },
    series: [{
      type: 'pie',
      radius: ['35%', '60%'],
      center: ['50%', '45%'],
      label: { formatter: '{b}\n{c}间', fontSize: 11 },
      data: roomStatusData.value,
    }],
  })
}

// 初始化收入趋势柱状图
async function initTrendChart() {
  if (!trendChartRef.value) return
  const echarts = await import('echarts')
  trendChart = echarts.init(trendChartRef.value)

  const paidData = billStore.bills
    .filter(b => b.status === '已支付')
    .reduce((acc, b) => {
      acc[b.year_month] = (acc[b.year_month] || 0) + b.total_amount
      return acc
    }, {} as Record<string, number>)

  const months = Object.keys(paidData).sort().slice(-6)
  const collected = months.map(m => toYuanNumber(paidData[m] || 0))
  const total = months.map(() => toYuanNumber(totalReceivable.value))

  trendChart.setOption({
    tooltip: { trigger: 'axis', formatter: (params: any[]) => `${params[0].name}<br/>已收: ¥${params[0].value?.toFixed(2) || 0}` },
    legend: { data: ['已收金额'], bottom: 0 },
    grid: { top: 10, bottom: 40, left: 60, right: 20 },
    xAxis: { type: 'category', data: months.map(m => m.replace('-', '年') + '月') },
    yAxis: { type: 'value', axisLabel: { formatter: (v: number) => `¥${(v / 10000).toFixed(1)}万` } },
    series: [{
      name: '已收金额',
      type: 'bar',
      data: collected,
      itemStyle: { color: '#409EFF' },
    }],
  })
}

async function loadData() {
  const ym = uiStore.globalYearMonth
  await Promise.all([
    roomStore.fetchRooms(),
    billStore.fetchBills(ym),
  ])
}

function handleResize() {
  progressChart?.resize()
  roomStatusChart?.resize()
  trendChart?.resize()
}
// 监听全局月份变化
watch(() => uiStore.globalYearMonth, async () => {
  if (!isMounted) return
  await loadData()
  if (isMounted) {
    await Promise.all([
      initProgressChart(),
      initRoomStatusChart(),
      initTrendChart(),
    ])
  }
})

onMounted(async () => {
  isMounted = true
  await loadData()
  await Promise.all([
    initProgressChart(),
    initRoomStatusChart(),
    initTrendChart(),
  ])
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  isMounted = false
  window.removeEventListener('resize', handleResize)
  progressChart?.dispose()
  roomStatusChart?.dispose()
  trendChart?.dispose()
})
</script>

<template>
  <div class="dashboard">
    <!-- 欢迎横幅 -->
    <div class="welcome-banner">
      <h2>欢迎回来！</h2>
      <p class="date">{{ currentDate }}</p>
    </div>

    <!-- 概览卡片 -->
    <el-row :gutter="16" class="overview-cards">
      <el-col :span="6" v-for="stat in stats" :key="stat.title">
        <el-card shadow="hover" class="stat-card">
          <el-statistic
            :title="stat.title"
            :value="stat.value"
            :precision="2"
          >
            <template v-if="stat.suffix" #suffix>{{ stat.suffix }}</template>
            <template v-if="stat.prefix" #prefix>{{ stat.prefix }}</template>
          </el-statistic>
          <div class="stat-footer">
            空房 {{ roomStore.vacantCount }} 间
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 图表区域 -->
    <el-row :gutter="16" class="chart-row">
      <!-- 收租进度环形图 -->
      <el-col :span="8">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <span>收租进度</span>
          </template>
          <div ref="progressChartRef" class="chart-container"></div>
          <div class="progress-legend">
            <div class="legend-item">
              <span class="dot collected"></span>
              <span>已收: {{ formatMoney(totalCollected) }}</span>
            </div>
            <div class="legend-item">
              <span class="dot unpaid"></span>
              <span>未收: {{ formatMoney(unpaidAmount) }}</span>
            </div>
          </div>
        </el-card>
      </el-col>

      <!-- 房间状态分布饼图 -->
      <el-col :span="8">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <span>房间状态分布</span>
          </template>
          <div ref="roomStatusChartRef" class="chart-container"></div>
        </el-card>
      </el-col>

      <!-- 收入趋势柱状图 -->
      <el-col :span="8">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <span>已收趋势（近6月）</span>
          </template>
          <div ref="trendChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 待办与快捷操作 -->
    <el-row :gutter="16" class="bottom-row">
      <!-- 待缴费提醒 -->
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>待缴费账单</span>
              <el-badge :value="billStore.pendingBills.length" type="danger" />
            </div>
          </template>
          <el-table
            v-if="pendingBills.length > 0"
            :data="pendingBills"
            size="small"
            :show-header="true"
          >
            <el-table-column label="房号" width="90">
              <template #default="{ row }">
                {{ roomStore.getRoomById(row.room_id)?.room_number || row.room_id }}室
              </template>
            </el-table-column>
            <el-table-column label="状态" width="80">
              <template #default="{ row }">
                <el-tag size="small" type="warning">{{ row.room_status }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="欠费金额" align="right">
              <template #default="{ row }">
                <span class="unpaid-amount">
                  {{ formatMoney(row.total_amount - row.actual_paid) }}
                </span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100">
              <template #default>
                <el-button link type="primary" @click="router.push('/payments')">
                  去缴费
                </el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-else description="暂无欠费" :image-size="60" />
        </el-card>
      </el-col>

      <!-- 快捷操作 -->
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <span>快捷操作</span>
          </template>
          <div class="quick-actions">
            <el-button type="primary" size="large" @click="router.push('/bills/generate')">
              <el-icon><Tickets /></el-icon>
              生成账单
            </el-button>
            <el-button type="success" size="large" @click="router.push('/payments')">
              <el-icon><Wallet /></el-icon>
              缴费登记
            </el-button>
            <el-button type="warning" size="large" @click="router.push('/rooms')">
              <el-icon><House /></el-icon>
              房态管理
            </el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<style scoped lang="scss">
.dashboard {
  padding: 20px;
}

.welcome-banner {
  margin-bottom: 20px;
  h2 { margin: 0 0 4px; }
  .date { color: var(--text-secondary); margin: 0; }
}

.overview-cards {
  margin-bottom: 16px;
  .stat-card {
    .stat-footer {
      margin-top: 8px;
      font-size: 12px;
      color: var(--text-secondary);
    }
  }
}

.chart-row {
  margin-bottom: 16px;
  .chart-card {
    height: 320px;
    .chart-container {
      height: 240px;
    }
    .progress-legend {
      display: flex;
      justify-content: center;
      gap: 20px;
      margin-top: 12px;
      .legend-item {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 13px;
        .dot {
          width: 10px;
          height: 10px;
          border-radius: 50%;
          &.collected { background: var(--donut-collected); }
          &.unpaid { background: var(--donut-unpaid); }
        }
      }
    }
  }
}

.bottom-row {
  margin-bottom: 16px;
  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .unpaid-amount {
    color: var(--color-danger);
    font-weight: bold;
  }
  .quick-actions {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    .el-button {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
      height: 56px;
    }
  }
}
</style>
